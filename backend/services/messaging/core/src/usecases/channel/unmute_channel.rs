use std::sync::Arc;

use chrono::Utc;
use domain::{
    channel::{ChannelError, id::ChannelId, repository::ChannelRepository},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::{id::UserId},
};

use crate::dto::requests::channel::unmute_channel::UnmuteChannelRequest;


async fn outranks_target<R: domain::server::role::ServerRoleRepository>(
    role_repository: &R,
    server_id: &domain::server::id::ServerId,
    actor: &UserId,
    target: &UserId,
) -> Result<bool, domain::shared::repository::error::RepositoryError> {
    let roles = role_repository.list_roles(server_id).await?;
    let assignments = role_repository.member_role_ids(server_id).await?;
    let top_of = |user: &UserId| {
        assignments
            .iter()
            .filter(|(uid, _)| uid == &user.uuid())
            .filter_map(|(_, rid)| roles.iter().find(|r| &r.id == rid))
            .map(|r| r.position)
            .max()
    };
    Ok(match (top_of(actor), top_of(target)) {
        (Some(a), Some(t)) => a > t,
        (Some(_), None) => true,
        _ => false,
    })
}

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

#[derive(Debug)]
pub struct UnmuteChannel<C, U, S, R, V>
where
    C: ChannelRepository,
    U: domain::user_profile::UserProfileRepository,
    S: domain::server::repository::ServerRepository,
    R: domain::server::role::ServerRoleRepository,
    V: crate::ports::voice_service::VoiceService,
{
    channel_repository: Arc<C>,
    user_repository: Arc<U>,
    server_repository: Arc<S>,
    role_repository: Arc<R>,
    voice_service: Arc<V>,
}

impl<C, U, S, R, V> UnmuteChannel<C, U, S, R, V>
where
    C: ChannelRepository,
    U: domain::user_profile::UserProfileRepository,
    S: domain::server::repository::ServerRepository,
    R: domain::server::role::ServerRoleRepository,
    V: crate::ports::voice_service::VoiceService,
{
    pub fn new(
        channel_repository: Arc<C>,
        user_repository: Arc<U>,
        server_repository: Arc<S>,
        role_repository: Arc<R>,
        voice_service: Arc<V>,
    ) -> Self {
        Self {
            channel_repository,
            user_repository,
            server_repository,
            role_repository,
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

        let server_id = domain::server::id::ServerId::new(request.server_id);
        let server_owner = self
            .server_repository
            .find_server_owner(&server_id)
            .await?;
        let is_server_owner = server_owner.as_ref() == Some(&initiator);
        let can_override = is_server_owner
            || (target != initiator
                && domain::server::role::has_permission(
                    self.role_repository
                        .user_permissions(&server_id, &initiator)
                        .await?,
                    domain::server::role::PERM_MUTE_MEMBERS,
                )
                && outranks_target(
                    self.role_repository.as_ref(),
                    &server_id,
                    &initiator,
                    &target,
                )
                .await?);
        let observed_muted_by = channel
            .muted_participants_list()
            .find(|m| m.user_id == target)
            .map(|m| m.muted_by);
        let moderator_record = observed_muted_by.is_some_and(|by| by != target);
        channel.unmute(target, initiator, can_override, now)?;
        let expected_muted_by = observed_muted_by.as_ref();
        let inherent = is_server_owner || channel.owner_id() == &initiator;
        let delegated = if !inherent && target != initiator {
            Some((&server_id, &initiator))
        } else {
            None
        };

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

        let persisted = self
            .channel_repository
            .unmute_participant_with_events(
                &channel_id,
                &target,
                expected_muted_by,
                delegated,
                &records,
            )
            .await;
        match persisted {
            Ok(true) => Ok(()),
            Ok(false) => {
                if moderator_record {
                    self.compensate_livekit(&channel_id, &target).await;
                }
                Err(UnmuteChannelError::Domain(ChannelError::NotOwner))
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
