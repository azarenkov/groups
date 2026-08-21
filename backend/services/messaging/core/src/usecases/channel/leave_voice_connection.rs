use std::sync::Arc;

use domain::{
    channel::{
        ChannelError,
        id::ChannelId,
        repository::{ChannelOperationError, ChannelRepository},
        session_id::SessionId,
    },
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::dto::requests::channel::leave_voice_connection::LeaveVoiceConnectionRequest;

#[derive(Debug, thiserror::Error)]
pub enum LeaveVoiceConnectionError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
}

impl From<ChannelOperationError> for LeaveVoiceConnectionError {
    fn from(error: ChannelOperationError) -> Self {
        match error {
            ChannelOperationError::Repository(e) => Self::Repository(e),
            ChannelOperationError::Domain(e) => Self::Domain(e),
        }
    }
}

#[derive(Debug)]
pub struct LeaveVoiceConnection<T>
where
    T: ChannelRepository,
{
    channel_repository: Arc<T>,
}

impl<T> LeaveVoiceConnection<T>
where
    T: ChannelRepository,
{
    pub fn new(channel_repository: Arc<T>) -> Self {
        Self { channel_repository }
    }

    pub async fn execute(
        &self,
        request: LeaveVoiceConnectionRequest,
    ) -> Result<bool, LeaveVoiceConnectionError> {
        let channel_id = ChannelId::new(request.channel_id);
        let user_id = UserId::new(request.user_id);
        let session_id = SessionId::new(request.session_id);

        let processed = self
            .channel_repository
            .apply_participant_left(
                &channel_id,
                user_id,
                session_id,
                request.reason,
                request.occurred_at,
                request.inbox_source.as_str(),
                &request.inbox_external_id,
                request.ctx.request_id,
            )
            .await?;

        Ok(processed)
    }
}
