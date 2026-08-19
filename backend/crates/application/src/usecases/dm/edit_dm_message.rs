use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::repository::AttachmentRepository,
    dm_message::{DmMessageError, id::DmMessageId, repository::DmMessageRepository},
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
use uuid::Uuid;

use crate::{
    dto::requests::dm::edit_dm_message::EditDmMessageRequest, ports::file_storage::FileStorage,
};

#[derive(Debug, thiserror::Error)]
pub enum EditDmMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("DM error: {0}")]
    Dm(#[from] DmMessageError),
    #[error("Message error: {0}")]
    Message(#[from] MessageError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Message not found")]
    NotFound,
    #[error("Invalid attachments")]
    InvalidAttachments,
}

pub struct EditDmMessage<M, A, FS>
where
    M: DmMessageRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    dm_repository: Arc<M>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
}

impl<M, A, FS> EditDmMessage<M, A, FS>
where
    M: DmMessageRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    pub fn new(dm_repository: Arc<M>, attachment_repository: Arc<A>, storage: Arc<FS>) -> Self {
        Self {
            dm_repository,
            attachment_repository,
            storage,
        }
    }

    pub async fn execute(
        &self,
        request: EditDmMessageRequest,
        editor_id: UserId,
    ) -> Result<(), EditDmMessageError> {
        let message_id = DmMessageId::new(request.message_id);

        let mut message = self
            .dm_repository
            .find_by_id(&message_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => EditDmMessageError::NotFound,
                other => EditDmMessageError::Repository(other),
            })?;

        if message.sender_id() != &editor_id {
            return Err(EditDmMessageError::NotFound);
        }

        let current = self
            .attachment_repository
            .list_for_dm_messages(&[message_id.uuid()])
            .await?;
        let current_ids: HashSet<Uuid> = current.iter().map(|a| a.id).collect();
        let remove_ids: HashSet<Uuid> = request.remove_attachment_ids.iter().copied().collect();
        if !remove_ids.iter().all(|id| current_ids.contains(id)) {
            return Err(EditDmMessageError::InvalidAttachments);
        }
        let add_ids: HashSet<Uuid> = request.add_attachment_ids.iter().copied().collect();
        if add_ids.iter().any(|id| current_ids.contains(id)) {
            return Err(EditDmMessageError::InvalidAttachments);
        }
        let added_attachments = if add_ids.is_empty() {
            Vec::new()
        } else {
            let owned = self
                .attachment_repository
                .list_unlinked_owned(&request.add_attachment_ids, &editor_id)
                .await?;
            if owned.len() != add_ids.len() {
                return Err(EditDmMessageError::InvalidAttachments);
            }
            owned
        };
        let remaining = current_ids.len() - remove_ids.len() + add_ids.len();
        if remaining > 10 {
            return Err(EditDmMessageError::InvalidAttachments);
        }

        let new_content = if remaining > 0 {
            MessageContent::new_allow_empty(request.content)?
        } else {
            MessageContent::new(request.content)?
        };
        let attachments_changed = !remove_ids.is_empty() || !add_ids.is_empty();
        let now = Utc::now();
        message.edit_with_options(new_content, editor_id, now, attachments_changed)?;
        let request_id = request.ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        let mut final_attachments: Vec<domain::attachment::Attachment> = current
            .iter()
            .filter(|a| !remove_ids.contains(&a.id))
            .cloned()
            .collect();
        final_attachments.extend(added_attachments);
        final_attachments.sort_by_key(|a| a.created_at);
        crate::usecases::enrichment::inject_field(
            &mut records,
            "attachments",
            crate::usecases::enrichment::attachments_value(&final_attachments),
        );
        crate::usecases::enrichment::inject_field(
            &mut records,
            "recipient_id",
            serde_json::json!(message.recipient_id().uuid()),
        );
        let removed_keys = self
            .dm_repository
            .update_with_events(
                &message,
                &records,
                &request.add_attachment_ids,
                &request.remove_attachment_ids,
            )
            .await
            .map_err(|e| match e {
                RepositoryError::InvalidData(_) => EditDmMessageError::InvalidAttachments,
                other => EditDmMessageError::Repository(other),
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
