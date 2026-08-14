use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::repository::AttachmentRepository,
    channel::{ChannelError, id::ChannelId, repository::ChannelRepository},
    message::{
        Message, MessageError, content::MessageContent, repository::MessageRepository,
    },
    server::{id::ServerId, repository::ServerRepository},
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
    requests::channel::send_channel_message::SendChannelMessageRequest,
    responses::channel::send_channel_message::SendChannelMessageResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum SendChannelMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message domain error: {0}")]
    Domain(#[from] MessageError),
    #[error("Channel domain error: {0}")]
    Channel(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Not a member of the server")]
    NotServerMember,
    #[error("Not a member of the channel")]
    NotChannelMember,
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Channel is not a text channel")]
    NotTextChannel,
    #[error("Invalid attachments")]
    InvalidAttachments,
}

pub struct SendChannelMessage<M, C, S, A>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
    A: AttachmentRepository,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
    attachment_repository: Arc<A>,
}

impl<M, C, S, A> SendChannelMessage<M, C, S, A>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
    A: AttachmentRepository,
{
    pub fn new(
        message_repository: Arc<M>,
        channel_repository: Arc<C>,
        server_repository: Arc<S>,
        attachment_repository: Arc<A>,
    ) -> Self {
        Self {
            message_repository,
            channel_repository,
            server_repository,
            attachment_repository,
        }
    }

    pub async fn execute(
        &self,
        request: SendChannelMessageRequest,
        author_id: UserId,
    ) -> Result<SendChannelMessageResponse, SendChannelMessageError> {
        let server_id = ServerId::new(request.server_id);
        let channel_id = ChannelId::new(request.channel_id);

        let is_member = self
            .server_repository
            .is_server_member(&server_id, &author_id)
            .await?;
        if !is_member {
            return Err(SendChannelMessageError::NotServerMember);
        }

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => SendChannelMessageError::ChannelNotFound,
                other => SendChannelMessageError::Repository(other),
            })?;

        if channel.server_id() != &server_id {
            return Err(SendChannelMessageError::ChannelNotFound);
        }

        if !channel.supports_text() {
            return Err(SendChannelMessageError::NotTextChannel);
        }

        let is_text_member = self
            .channel_repository
            .is_text_channel_member(&channel_id, &author_id)
            .await?;
        if !is_text_member {
            return Err(SendChannelMessageError::NotChannelMember);
        }

        if request.attachment_ids.len() > 10 {
            return Err(SendChannelMessageError::InvalidAttachments);
        }
        let attachments = if request.attachment_ids.is_empty() {
            Vec::new()
        } else {
            let owned = self
                .attachment_repository
                .list_unlinked_owned(&request.attachment_ids, &author_id)
                .await?;
            let distinct: std::collections::HashSet<&uuid::Uuid> =
                request.attachment_ids.iter().collect();
            if owned.len() != distinct.len() {
                return Err(SendChannelMessageError::InvalidAttachments);
            }
            owned
        };

        let content = if request.attachment_ids.is_empty() {
            MessageContent::new(request.content)?
        } else {
            MessageContent::new_allow_empty(request.content)?
        };
        let now = Utc::now();
        let mut message = Message::create(channel_id, author_id, content, now);

        let recipient_ids = self
            .channel_repository
            .list_text_channel_member_ids(&channel_id)
            .await?;

        let request_id = request.ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        crate::usecases::channel::recipients::inject_recipient_ids(&mut records, &recipient_ids);

        self.message_repository
            .save_message_with_events(&message, &records, &request.attachment_ids)
            .await?;

        let mut response = SendChannelMessageResponse::from(&message);
        response.attachments = attachments;
        Ok(response)
    }
}
