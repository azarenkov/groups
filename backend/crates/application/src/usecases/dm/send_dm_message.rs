use std::sync::Arc;

use chrono::Utc;
use domain::{
    dm_message::{DmMessage, repository::DmMessageRepository},
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    message::{MessageError, content::MessageContent},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::dto::{
    requests::dm::send_dm_message::SendDmMessageRequest,
    responses::dm::send_dm_message::SendDmMessageResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum SendDmMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
    #[error("Not friends with this user")]
    NotFriends,
}

pub struct SendDmMessage<M, F>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
}

impl<M, F> SendDmMessage<M, F>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
{
    pub fn new(dm_repository: Arc<M>, friendship_repository: Arc<F>) -> Self {
        Self {
            dm_repository,
            friendship_repository,
        }
    }

    pub async fn execute(
        &self,
        request: SendDmMessageRequest,
        sender_id: UserId,
    ) -> Result<SendDmMessageResponse, SendDmMessageError> {
        let recipient_id = UserId::new(request.recipient_id);

        let pair = self
            .friendship_repository
            .find_pair(&sender_id, &recipient_id)
            .await?;
        if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
            return Err(SendDmMessageError::NotFriends);
        }

        let content = MessageContent::new(request.content)?;
        let now = Utc::now();
        let message = DmMessage::create(sender_id, recipient_id, content, now);
        self.dm_repository.save(&message).await?;
        Ok(SendDmMessageResponse::from(&message))
    }
}
