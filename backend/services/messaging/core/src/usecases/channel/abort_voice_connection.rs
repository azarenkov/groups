use std::sync::Arc;

use messaging_domain::{channel::ChannelError, channel::id::ChannelId, channel::session_id::SessionId, channel::unit_of_work::ChannelUnitOfWork, events::MessagingEvent};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::dto::requests::channel::abort_voice_connection::AbortVoiceConnectionRequest;

#[derive(Debug, thiserror::Error)]
pub enum AbortVoiceConnectionError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
}

#[derive(Debug)]
pub struct AbortVoiceConnection<W: ChannelUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: ChannelUnitOfWork> AbortVoiceConnection<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(&self, request: AbortVoiceConnectionRequest) -> Result<bool, AbortVoiceConnectionError> {
        let channel_id = ChannelId::new(request.channel_id);
        let user_id = UserId::new(request.user_id);
        let session_id = SessionId::new(request.session_id);

        let mut tx = self.unit_of_work.begin().await?;
        if !tx
            .claim_inbox(request.inbox_source.as_str(), &request.inbox_external_id)
            .await?
        {
            return Ok(false);
        }
        let mut channel = tx.load_channel_for_update(&channel_id).await?;
        channel.connection_aborted(user_id, session_id, request.reason, request.occurred_at)?;
        let events = channel.pull_events();
        tx.save_voice_state(&mut channel, &events, request.ctx.request_id.as_deref())
            .await?;
        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(true)
    }
}
