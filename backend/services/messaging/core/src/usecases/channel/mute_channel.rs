use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{channel::ChannelError, channel::id::ChannelId, channel::repository::ChannelRepository};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};

use crate::dto::requests::channel::mute_channel::MuteChannelRequest;


#[derive(Debug, thiserror::Error)]
pub enum MuteChannelError {
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

impl From<messaging_domain::server::policy::PolicyError> for MuteChannelError {
    fn from(err: messaging_domain::server::policy::PolicyError) -> Self {
        match err {
            messaging_domain::server::policy::PolicyError::TargetNotMember => MuteChannelError::TargetUserNotFound,
            _ => MuteChannelError::Domain(ChannelError::NotOwner),
        }
    }
}

#[derive(Debug)]
pub struct MuteChannel<C, U, W, V>
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

impl<C, U, W, V> MuteChannel<C, U, W, V>
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

    async fn compensate_livekit(
        &self,
        channel_id: &ChannelId,
        target: &UserId,
        muted_sids: &[String],
    ) {
        let committed = match self.channel_repository.find_channel_by_id(channel_id).await {
            Ok(channel) => channel
                .muted_participants_list()
                .find(|m| &m.user_id == target)
                .is_some_and(|m| &m.muted_by != target),
            Err(_) => true,
        };
        if committed {
            return;
        }
        if let Err(err) = self
            .voice_service
            .unmute_tracks(channel_id, target, muted_sids)
            .await
        {
            tracing::warn!(
                channel_id = %channel_id.uuid(),
                user_id = %target.uuid(),
                error = %err,
                "Failed to revert LiveKit track mutes during compensation",
            );
        }
        if let Err(err) = self.voice_service.unmute_participant(channel_id, target).await {
            tracing::warn!(
                channel_id = %channel_id.uuid(),
                user_id = %target.uuid(),
                error = %err,
                "Failed to restore LiveKit publication rights during compensation",
            );
        }
    }

    pub async fn execute(
        &self,
        request: MuteChannelRequest,
        initiator: UserId,
    ) -> Result<(), MuteChannelError> {
        let channel_id = ChannelId::new(request.channel_id);
        let target = UserId::new(request.target_user_id);

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id().uuid() != request.server_id {
            return Err(MuteChannelError::ChannelNotFound);
        }

        if !channel.supports_voice() {
            return Err(MuteChannelError::VoiceNotSupported);
        }

        self.user_repository
            .get_name(&target)
            .await?
            .ok_or(MuteChannelError::TargetUserNotFound)?;

        let now = Utc::now();

        let server_id = messaging_domain::server::id::ServerId::new(request.server_id);
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        messaging_domain::server::policy::ModerationPolicy::mute(&snapshot, &initiator, &target)?;
        let moderator_mute = target != initiator;
        channel.mute(target, initiator, moderator_mute, now)?;

        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        let muted_sids = if moderator_mute {
            self.voice_service
                .mute_participant(&channel_id, &target)
                .await?
        } else {
            Vec::new()
        };

        let persisted = async {
            if !tx
                .mute_participant(&channel_id, &target, &initiator, now, !moderator_mute)
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
                if moderator_mute {
                    self.compensate_livekit(&channel_id, &target, &muted_sids).await;
                }
                Err(MuteChannelError::Domain(ChannelError::AlreadyMuted))
            }
            Err(err) => {
                if moderator_mute {
                    self.compensate_livekit(&channel_id, &target, &muted_sids).await;
                }
                Err(err.into())
            }
        }
    }
}
