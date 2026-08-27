use std::sync::Arc;

use domain::shared::outbox::to_outbox::ToOutboxRecord;

use messaging_domain::{channel::repository::ChannelRepository, server::id::ServerId, server::unit_of_work::ServerUnitOfWork};
use domain::{shared::repository::error::RepositoryError, user::id::UserId};

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

pub struct LeaveServer<W, C, V>
where
    W: ServerUnitOfWork,
    C: ChannelRepository,
    V: VoiceService,
{
    unit_of_work: Arc<W>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<W, C, V> LeaveServer<W, C, V>
where
    W: ServerUnitOfWork,
    C: ChannelRepository,
    V: VoiceService,
{
    pub fn new(unit_of_work: Arc<W>, channel_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self {
            unit_of_work,
            channel_repository,
            voice_service,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
    ) -> Result<(), LeaveServerError> {
        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_server(&server_id)
            .await?;

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        if snapshot.is_owner(&user_id) {
            return Err(LeaveServerError::OwnerCannotLeave);
        }

        let member_event = messaging_domain::server::events::ServerEvent::MemberLeft(
            messaging_domain::server::events::lifecycle::ServerMemberLeft::record(
                server_id,
                user_id,
                chrono::Utc::now(),
            ),
        );
        if !tx.remove_member(&user_id).await? {
            return Err(LeaveServerError::NotMember);
        }
        tx.save_outbox(&[member_event.to_outbox_record()?]).await?;
        tx.commit().await?;

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
