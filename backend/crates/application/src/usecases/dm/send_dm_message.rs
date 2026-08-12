use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::repository::AttachmentRepository,
    dm_message::{DmMessage, repository::DmMessageRepository},
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    message::{MessageError, content::MessageContent},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
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
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Not friends with this user")]
    NotFriends,
    #[error("Invalid attachments")]
    InvalidAttachments,
}

pub struct SendDmMessage<M, F, A>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    A: AttachmentRepository,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
    attachment_repository: Arc<A>,
}

impl<M, F, A> SendDmMessage<M, F, A>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    A: AttachmentRepository,
{
    pub fn new(
        dm_repository: Arc<M>,
        friendship_repository: Arc<F>,
        attachment_repository: Arc<A>,
    ) -> Self {
        Self {
            dm_repository,
            friendship_repository,
            attachment_repository,
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

        if request.attachment_ids.len() > 10 {
            return Err(SendDmMessageError::InvalidAttachments);
        }
        if !request.attachment_ids.is_empty() {
            let owned = self
                .attachment_repository
                .count_unlinked_owned(&request.attachment_ids, &sender_id)
                .await?;
            if owned as usize != request.attachment_ids.len() {
                return Err(SendDmMessageError::InvalidAttachments);
            }
        }

        let content = if request.attachment_ids.is_empty() {
            MessageContent::new(request.content)?
        } else {
            MessageContent::new_allow_empty(request.content)?
        };
        let now = Utc::now();
        let mut message = DmMessage::create(sender_id, recipient_id, content, now);
        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        self.dm_repository
            .save_with_events(&message, &records, &request.attachment_ids)
            .await?;

        let mut response = SendDmMessageResponse::from(&message);
        if !request.attachment_ids.is_empty() {
            response.attachments = self
                .attachment_repository
                .list_for_dm_messages(&[message.id().uuid()])
                .await?;
        }
        Ok(response)
    }
}
