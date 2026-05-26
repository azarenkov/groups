use std::sync::Arc;

use chrono::Utc;
use domain::{
    message::{
        MessageError, content::MessageContent, id::MessageId, repository::MessageRepository,
    },
    channel::{id::ChannelId, repository::ChannelRepository},
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

use crate::dto::requests::channel::edit_channel_message::EditChannelMessageRequest;

#[derive(Debug, thiserror::Error)]
pub enum EditChannelMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message domain error: {0}")]
    Domain(#[from] MessageError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Not a member of the server")]
    NotServerMember,
    #[error("Message not found")]
    MessageNotFound,
}

pub struct EditChannelMessage<M, C, S>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
}

impl<M, C, S> EditChannelMessage<M, C, S>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
{
    pub fn new(
        message_repository: Arc<M>,
        channel_repository: Arc<C>,
        server_repository: Arc<S>,
    ) -> Self {
        Self {
            message_repository,
            channel_repository,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        request: EditChannelMessageRequest,
        editor_id: UserId,
    ) -> Result<(), EditChannelMessageError> {
        let server_id = ServerId::new(request.server_id);
        let channel_id = ChannelId::new(request.channel_id);
        let message_id = MessageId::new(request.message_id);

        let is_member = self
            .server_repository
            .is_server_member(&server_id, &editor_id)
            .await?;
        if !is_member {
            return Err(EditChannelMessageError::NotServerMember);
        }

        let mut message = self
            .message_repository
            .find_message_by_id(&message_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => EditChannelMessageError::MessageNotFound,
                other => EditChannelMessageError::Repository(other),
            })?;

        if message.channel_id().uuid() != channel_id.uuid() {
            return Err(EditChannelMessageError::MessageNotFound);
        }

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => EditChannelMessageError::MessageNotFound,
                other => EditChannelMessageError::Repository(other),
            })?;

        if channel.server_id() != &server_id {
            return Err(EditChannelMessageError::MessageNotFound);
        }

        let new_content = MessageContent::new(request.new_content)?;
        let now = Utc::now();
        message.edit(new_content, editor_id, now)?;

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
            .update_message_with_events(&message, &records)
            .await?;

        Ok(())
    }
}
