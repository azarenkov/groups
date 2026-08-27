use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{channel::ChannelError, channel::id::ChannelId, channel::repository::ChannelRepository};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};

use crate::dto::requests::channel::unmute_channel::UnmuteChannelRequest;


#[derive(Debug, thiserror::Error)]
pub enum UnmuteChannelError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Target user not found")]
    TargetUserNotFound,
    #[error("Voice not supported for this channel")]
    VoiceNotSupported,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] crate::ports::voice_service::VoiceServiceError),
}

impl From<messaging_domain::server::policy::PolicyError> for UnmuteChannelError {
    fn from(err: messaging_domain::server::policy::PolicyError) -> Self {
        match err {
            messaging_domain::server::policy::PolicyError::TargetNotMember => UnmuteChannelError::TargetUserNotFound,
            _ => UnmuteChannelError::Domain(ChannelError::NotOwner),
        }
    }
}

#[derive(Debug)]
pub struct UnmuteChannel<C, U, W, V>
where
    C: ChannelRepository,
    U: domain::user_profile::UserProfileRepository,
    W: messaging_domain::server::unit_of_work::ServerUnitOfWork,
    V: crate::ports::voice_service::VoiceService,
{
    channel_repository: Arc<C>,
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    voice_service: Arc<V>,
}

impl<C, U, W, V> UnmuteChannel<C, U, W, V>
where
    C: ChannelRepository,
    U: domain::user_profile::UserProfileRepository,
    W: messaging_domain::server::unit_of_work::ServerUnitOfWork,
    V: crate::ports::voice_service::VoiceService,
{
    pub fn new(
        channel_repository: Arc<C>,
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        voice_service: Arc<V>,
    ) -> Self {
        Self {
            channel_repository,
            user_repository,
            unit_of_work,
            voice_service,
        }
    }

    async fn compensate_livekit(&self, channel_id: &ChannelId, target: &UserId) {
        let still_muted = match self.channel_repository.find_channel_by_id(channel_id).await {
            Ok(channel) => channel
                .muted_participants_list()
                .find(|m| &m.user_id == target)
                .is_some_and(|m| &m.muted_by != target),
            Err(_) => true,
        };
        if still_muted {
            let _ = self.voice_service.mute_participant(channel_id, target).await;
        }
    }

    pub async fn execute(
        &self,
        request: UnmuteChannelRequest,
        initiator: UserId,
    ) -> Result<(), UnmuteChannelError> {
        let channel_id = ChannelId::new(request.channel_id);
        let target = UserId::new(request.target_user_id);

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id().uuid() != request.server_id {
            return Err(UnmuteChannelError::ChannelNotFound);
        }

        if !channel.supports_voice() {
            return Err(UnmuteChannelError::VoiceNotSupported);
        }

        self.user_repository
            .get_name(&target)
            .await?
            .ok_or(UnmuteChannelError::TargetUserNotFound)?;

        let now = Utc::now();

        let server_id = messaging_domain::server::id::ServerId::new(request.server_id);
        let observed_muted_by = channel
            .muted_participants_list()
            .find(|m| m.user_id == target)
            .map(|m| m.muted_by);
        let moderator_record = observed_muted_by.is_some_and(|by| by != target);

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        messaging_domain::server::policy::ModerationPolicy::unmute(
            &snapshot,
            &initiator,
            &target,
            observed_muted_by.as_ref(),
        )?;
        let can_override = snapshot.is_owner(&initiator) || target != initiator;
        let outcome = channel.unmute(target, initiator, can_override, now)?;
        let downgrade = outcome == messaging_domain::channel::UnmuteOutcome::DowngradedToSelfMute;

        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        if moderator_record {
            self.voice_service
                .unmute_participant(&channel_id, &target)
                .await?;
        }

        let persisted = async {
            if !tx
                .unmute_participant(&channel_id, &target, observed_muted_by.as_ref(), downgrade)
                .await?
            {
                return Ok::<bool, RepositoryError>(false);
            }
            tx.save_outbox(&records).await?;
            tx.commit().await?;
            Ok(true)
        }
        .await;
        match persisted {
            Ok(true) => Ok(()),
            Ok(false) => {
                if moderator_record {
                    self.compensate_livekit(&channel_id, &target).await;
                }
                Err(UnmuteChannelError::Domain(ChannelError::NotMuted))
            }
            Err(err) => {
                if moderator_record {
                    self.compensate_livekit(&channel_id, &target).await;
                }
                Err(err.into())
            }
        }
    }
}
