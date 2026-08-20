use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use domain::{
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
    dto::requests::dm::edit_dm_message::EditDmMessageRequest,
    ports::attachment_claims::{AttachmentClaimError, AttachmentClaims},
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
    #[error("Attachment service unavailable: {0}")]
    AttachmentsUnavailable(String),
}

pub struct EditDmMessage<M, A>
where
    M: DmMessageRepository,
    A: AttachmentClaims,
{
    dm_repository: Arc<M>,
    attachment_claims: Arc<A>,
}

impl<M, A> EditDmMessage<M, A>
where
    M: DmMessageRepository,
    A: AttachmentClaims,
{
    pub fn new(dm_repository: Arc<M>, attachment_claims: Arc<A>) -> Self {
        Self {
            dm_repository,
            attachment_claims,
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

        let current: Vec<domain::attachment::AttachmentLink> = self
            .dm_repository
            .list_attachment_links_for_dm_messages(&[message_id.uuid()])
            .await?
            .into_iter()
            .map(|(_, link)| link)
            .collect();
        let current_ids: HashSet<Uuid> = current.iter().map(|a| a.id).collect();
        let remove_ids: HashSet<Uuid> = request.remove_attachment_ids.iter().copied().collect();
        if !remove_ids.iter().all(|id| current_ids.contains(id)) {
            return Err(EditDmMessageError::InvalidAttachments);
        }
        let add_ids: HashSet<Uuid> = request.add_attachment_ids.iter().copied().collect();
        if add_ids.iter().any(|id| current_ids.contains(id)) {
            return Err(EditDmMessageError::InvalidAttachments);
        }
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
        let claim_token = uuid::Uuid::new_v4();
        let added_attachments = if add_ids.is_empty() {
            Vec::new()
        } else {
            match self
                .attachment_claims
                .claim(&request.add_attachment_ids, &editor_id, claim_token)
                .await
            {
                Ok(links) => links,
                Err(AttachmentClaimError::Invalid) => {
                    return Err(EditDmMessageError::InvalidAttachments);
                }
                Err(AttachmentClaimError::Unavailable(e)) => {
                    if let Err(re) = self
                        .attachment_claims
                        .release(&request.add_attachment_ids, claim_token)
                        .await
                    {
                        tracing::warn!(error = %re, "Failed to release attachments after ambiguous claim failure");
                    }
                    return Err(EditDmMessageError::AttachmentsUnavailable(e));
                }
            }
        };
        let mut final_attachments: Vec<domain::attachment::AttachmentLink> = current
            .iter()
            .filter(|a| !remove_ids.contains(&a.id))
            .cloned()
            .collect();
        final_attachments.extend(added_attachments.clone());
        final_attachments.sort_by_key(|a| a.created_at);
        crate::usecases::enrichment::inject_field(
            &mut records,
            "attachments",
            crate::usecases::enrichment::attachment_links_value(&final_attachments),
        );
        crate::usecases::enrichment::inject_field(
            &mut records,
            "recipient_id",
            serde_json::json!(message.recipient_id().uuid()),
        );
        if let Err(e) = self
            .dm_repository
            .update_with_events(
                &message,
                &records,
                &added_attachments,
                &request.remove_attachment_ids,
            )
            .await
        {
            if !request.add_attachment_ids.is_empty()
                && let Err(re) = self
                    .attachment_claims
                    .release(&request.add_attachment_ids, claim_token)
                    .await
            {
                tracing::warn!(error = %re, "Failed to release attachment claims after edit failure");
            }
            return Err(match e {
                RepositoryError::InvalidData(_) => EditDmMessageError::InvalidAttachments,
                other => EditDmMessageError::Repository(other),
            });
        }
        Ok(())
    }
}
