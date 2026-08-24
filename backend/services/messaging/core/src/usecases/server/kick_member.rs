use std::sync::Arc;

use domain::shared::outbox::to_outbox::ToOutboxRecord;

use domain::{
    channel::repository::ChannelRepository,
    server::{
        id::ServerId,
        repository::ServerRepository,
        role::{PERM_KICK_MEMBERS, ServerRoleRepository, has_permission},
    },
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::ports::voice_service::VoiceService;

#[derive(Debug, thiserror::Error)]
pub enum KickMemberError {
    #[error("Outbox error: {0}")]
    Outbox(#[from] domain::shared::outbox::to_outbox::OutboxError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Target is not a member of this server")]
    NotMember,
    #[error("Missing permission to kick members")]
    NotAllowed,
}

pub struct KickMember<S, C, V, R>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
    R: ServerRoleRepository,
{
    server_repository: Arc<S>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
    role_repository: Arc<R>,
}

impl<S, C, V, R> KickMember<S, C, V, R>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
    R: ServerRoleRepository,
{
    pub fn new(
        server_repository: Arc<S>,
        channel_repository: Arc<C>,
        voice_service: Arc<V>,
        role_repository: Arc<R>,
    ) -> Self {
        Self {
            server_repository,
            channel_repository,
            voice_service,
            role_repository,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        target: UserId,
    ) -> Result<(), KickMemberError> {
        if actor == target {
            return Err(KickMemberError::NotAllowed);
        }
        let owner = self.server_repository.find_server_owner(&server_id).await?;
        if owner.as_ref() == Some(&target) {
            return Err(KickMemberError::NotAllowed);
        }
        let is_owner = owner.as_ref() == Some(&actor);
        if !is_owner {
            let perms = self
                .role_repository
                .user_permissions(&server_id, &actor)
                .await?;
            if !has_permission(perms, PERM_KICK_MEMBERS) {
                return Err(KickMemberError::NotAllowed);
            }
            let roles = self.role_repository.list_roles(&server_id).await?;
            let assignments = self.role_repository.member_role_ids(&server_id).await?;
            let top_of = |user: &UserId| {
                assignments
                    .iter()
                    .filter(|(uid, _)| uid == &user.uuid())
                    .filter_map(|(_, rid)| roles.iter().find(|r| &r.id == rid))
                    .map(|r| r.position)
                    .max()
            };
            let actor_top = top_of(&actor);
            let target_top = top_of(&target);
            let outranks = match (actor_top, target_top) {
                (Some(a), Some(t)) => a > t,
                (Some(_), None) => true,
                _ => false,
            };
            if !outranks {
                return Err(KickMemberError::NotAllowed);
            }
        }

        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_server(&server_id)
            .await?;

        let member_event = domain::server::events::ServerEvent::MemberLeft(
            domain::server::events::lifecycle::ServerMemberLeft::record(
                server_id,
                target,
                chrono::Utc::now(),
            ),
        );
        let member_records = vec![member_event.to_outbox_record()?];
        let removed = self
            .server_repository
            .remove_server_member(&server_id, &target, &member_records)
            .await?;
        if !removed {
            return Err(KickMemberError::NotMember);
        }

        for channel_id in &voice_channel_ids {
            if let Err(err) = self
                .voice_service
                .remove_participant(channel_id, &target)
                .await
            {
                tracing::warn!(
                    channel_id = %channel_id.uuid(),
                    user_id = %target.uuid(),
                    error = %err,
                    "Failed to disconnect LiveKit participant after kick",
                );
            }
        }

        Ok(())
    }
}
