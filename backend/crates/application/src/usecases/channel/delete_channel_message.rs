use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::repository::AttachmentRepository,
    channel::{id::ChannelId, repository::ChannelRepository},
    message::{MessageError, id::MessageId, repository::MessageRepository},
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

use crate::{
    dto::requests::channel::delete_channel_message::DeleteChannelMessageRequest,
    ports::file_storage::FileStorage,
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteChannelMessageError {
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

pub struct DeleteChannelMessage<M, C, S, A, FS>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
}

impl<M, C, S, A, FS> DeleteChannelMessage<M, C, S, A, FS>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    pub fn new(
        message_repository: Arc<M>,
        channel_repository: Arc<C>,
        server_repository: Arc<S>,
        attachment_repository: Arc<A>,
        storage: Arc<FS>,
    ) -> Self {
        Self {
            message_repository,
            channel_repository,
            server_repository,
            attachment_repository,
            storage,
        }
    }

    pub async fn execute(
        &self,
        request: DeleteChannelMessageRequest,
        deleter_id: UserId,
    ) -> Result<(), DeleteChannelMessageError> {
        let server_id = ServerId::new(request.server_id);
        let channel_id = ChannelId::new(request.channel_id);
        let message_id = MessageId::new(request.message_id);

        let is_member = self
            .server_repository
            .is_server_member(&server_id, &deleter_id)
            .await?;
        if !is_member {
            return Err(DeleteChannelMessageError::NotServerMember);
        }

        let mut message = self
            .message_repository
            .find_message_by_id(&message_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => DeleteChannelMessageError::MessageNotFound,
                other => DeleteChannelMessageError::Repository(other),
            })?;

        if message.channel_id().uuid() != channel_id.uuid() {
            return Err(DeleteChannelMessageError::MessageNotFound);
        }

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => DeleteChannelMessageError::MessageNotFound,
                other => DeleteChannelMessageError::Repository(other),
            })?;

        if channel.server_id() != &server_id {
            return Err(DeleteChannelMessageError::MessageNotFound);
        }

        let is_owner = channel.owner_id() == &deleter_id;

        let now = Utc::now();
        message.mark_deleted(deleter_id, is_owner, now)?;

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

        let storage_keys = self
            .message_repository
            .delete_message_with_events(&message_id, &records)
            .await?;
        for key in storage_keys {
            match self.storage.delete(&key).await {
                Ok(()) | Err(crate::ports::file_storage::FileStorageError::NotFound) => {
                    if let Err(e) = self.attachment_repository.dequeue_cleanup(&key).await {
                        tracing::warn!(error = %e, %key, "Failed to dequeue cleaned attachment");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, %key, "Failed to delete attachment object; cleanup queue will retry");
                }
            }
        }

        Ok(())
    }
}
