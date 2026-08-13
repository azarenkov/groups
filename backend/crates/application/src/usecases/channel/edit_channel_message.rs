use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::repository::AttachmentRepository,
    channel::{id::ChannelId, repository::ChannelRepository},
    message::{
        MessageError, content::MessageContent, id::MessageId, repository::MessageRepository,
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
use uuid::Uuid;

use crate::{
    dto::requests::channel::edit_channel_message::EditChannelMessageRequest,
    ports::file_storage::FileStorage,
};

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
    #[error("Invalid attachments")]
    InvalidAttachments,
}

pub struct EditChannelMessage<M, C, S, A, FS>
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

impl<M, C, S, A, FS> EditChannelMessage<M, C, S, A, FS>
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

        let current = self
            .attachment_repository
            .list_for_messages(&[message_id.uuid()])
            .await?;
        let current_ids: HashSet<Uuid> = current.iter().map(|a| a.id).collect();
        let remove_ids: HashSet<Uuid> = request.remove_attachment_ids.iter().copied().collect();
        if !remove_ids.iter().all(|id| current_ids.contains(id)) {
            return Err(EditChannelMessageError::InvalidAttachments);
        }
        let add_ids: HashSet<Uuid> = request.add_attachment_ids.iter().copied().collect();
        if add_ids.iter().any(|id| current_ids.contains(id)) {
            return Err(EditChannelMessageError::InvalidAttachments);
        }
        if !add_ids.is_empty() {
            let owned = self
                .attachment_repository
                .list_unlinked_owned(&request.add_attachment_ids, &editor_id)
                .await?;
            if owned.len() != add_ids.len() {
                return Err(EditChannelMessageError::InvalidAttachments);
            }
        }
        let remaining = current_ids.len() - remove_ids.len() + add_ids.len();
        if remaining > 10 {
            return Err(EditChannelMessageError::InvalidAttachments);
        }

        let new_content = if remaining > 0 {
            MessageContent::new_allow_empty(request.new_content)?
        } else {
            MessageContent::new(request.new_content)?
        };
        let attachments_changed = !remove_ids.is_empty() || !add_ids.is_empty();
        let now = Utc::now();
        message.edit_with_options(new_content, editor_id, now, attachments_changed)?;

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

        let removed_keys = self
            .message_repository
            .update_message_with_events(
                &message,
                &records,
                &request.add_attachment_ids,
                &request.remove_attachment_ids,
            )
            .await
            .map_err(|e| match e {
                RepositoryError::InvalidData(_) => EditChannelMessageError::InvalidAttachments,
                other => EditChannelMessageError::Repository(other),
            })?;

        for key in removed_keys {
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
