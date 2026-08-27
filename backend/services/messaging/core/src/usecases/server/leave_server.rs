use std::sync::Arc;

use domain::shared::outbox::to_outbox::ToOutboxRecord;

use domain::{
    channel::repository::ChannelRepository,
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::ports::voice_service::VoiceService;

#[derive(Debug, thiserror::Error)]
pub enum LeaveServerError {
    #[error("Outbox error: {0}")]
    Outbox(#[from] domain::shared::outbox::to_outbox::OutboxError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this server")]
    NotMember,
    #[error("Server owners cannot leave their own server; delete it instead")]
    OwnerCannotLeave,
}

pub struct LeaveServer<S, C, V>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
{
    server_repository: Arc<S>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<S, C, V> LeaveServer<S, C, V>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
{
    pub fn new(
        server_repository: Arc<S>,
        channel_repository: Arc<C>,
        voice_service: Arc<V>,
    ) -> Self {
        Self {
            server_repository,
            channel_repository,
            voice_service,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
    ) -> Result<(), LeaveServerError> {
        let owner = self.server_repository.find_server_owner(&server_id).await?;
        if owner.as_ref() == Some(&user_id) {
            return Err(LeaveServerError::OwnerCannotLeave);
        }

        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_server(&server_id)
            .await?;

        let member_event = domain::server::events::ServerEvent::MemberLeft(
            domain::server::events::lifecycle::ServerMemberLeft::record(
                server_id,
                user_id,
                chrono::Utc::now(),
            ),
        );
        let member_records = vec![member_event.to_outbox_record()?];
        let removed = self
            .server_repository
            .remove_server_member(&server_id, &user_id, None, &member_records)
            .await?;
        if removed != domain::server::repository::RemoveMemberOutcome::Removed {
            return Err(LeaveServerError::NotMember);
        }

        for channel_id in &voice_channel_ids {
            if let Err(err) = self
                .voice_service
                .remove_participant(channel_id, &user_id)
                .await
            {
                tracing::warn!(
                    channel_id = %channel_id.uuid(),
                    user_id = %user_id.uuid(),
                    error = %err,
                    "Failed to disconnect LiveKit participant after leave",
                );
            }
        }

        Ok(())
    }
}
