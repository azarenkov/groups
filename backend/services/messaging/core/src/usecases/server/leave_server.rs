use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;

use messaging_domain::{events::MessagingEvent, channel::repository::ChannelRepository, server::id::ServerId, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};

use crate::ports::voice_service::VoiceService;

#[derive(Debug, thiserror::Error)]
pub enum LeaveServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this server")]
    NotMember,
    #[error("Server owners cannot leave their own server; delete it instead")]
    OwnerCannotLeave,
}

impl From<PolicyError> for LeaveServerError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => LeaveServerError::NotMember,
            _ => LeaveServerError::OwnerCannotLeave,
        }
    }
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
        let mut server = tx.load_server().await?;
        server.leave(&user_id, chrono::Utc::now())?;
        if !tx.remove_member(&user_id).await? {
            return Err(LeaveServerError::NotMember);
        }
        let events: Vec<MessagingEvent> = server.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, None).await?;
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
