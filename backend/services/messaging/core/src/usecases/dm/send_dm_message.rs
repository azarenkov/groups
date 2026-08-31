use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{privacy::DmPolicy, privacy::PrivacyRepository, events::MessagingEvent, dm_message::DmMessage, dm_message::repository::DmMessageRepository, dm_message::unit_of_work::DmUnitOfWork, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::MessageError, message::content::MessageContent};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};
use ports::repository::RepositoryError;

use crate::{
    dto::{
        requests::dm::send_dm_message::SendDmMessageRequest,
        responses::dm::send_dm_message::SendDmMessageResponse,
    },
    ports::attachment_claims::{AttachmentClaimError, AttachmentClaims},
};

#[derive(Debug, thiserror::Error)]
pub enum SendDmMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
    #[error("Not friends with this user")]
    NotFriends,
    #[error("This user does not accept direct messages")]
    DmsClosed,
    #[error("Cannot send a direct message to yourself")]
    SelfDm,
    #[error("Invalid attachments")]
    InvalidAttachments,
    #[error("Attachment service unavailable: {0}")]
    AttachmentsUnavailable(String),
}

pub struct SendDmMessage<M, W, F, A, P>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
    F: FriendshipRepository,
    A: AttachmentClaims,
    P: PrivacyRepository,
{
    dm_repository: Arc<M>,
    unit_of_work: Arc<W>,
    friendship_repository: Arc<F>,
    attachment_claims: Arc<A>,
    privacy_repository: Arc<P>,
}

impl<M, W, F, A, P> SendDmMessage<M, W, F, A, P>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
    F: FriendshipRepository,
    A: AttachmentClaims,
    P: PrivacyRepository,
{
    pub fn new(
        dm_repository: Arc<M>,
        unit_of_work: Arc<W>,
        friendship_repository: Arc<F>,
        attachment_claims: Arc<A>,
        privacy_repository: Arc<P>,
    ) -> Self {
        Self {
            dm_repository,
            unit_of_work,
            friendship_repository,
            attachment_claims,
            privacy_repository,
        }
    }

    pub async fn execute(
        &self,
        request: SendDmMessageRequest,
        sender_id: UserId,
    ) -> Result<SendDmMessageResponse, SendDmMessageError> {
        let recipient_id = UserId::new(request.recipient_id);

        if recipient_id == sender_id {
            return Err(SendDmMessageError::SelfDm);
        }
        let pair = self
            .friendship_repository
            .find_pair(&sender_id, &recipient_id)
            .await?;
        let are_friends = matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted);
        let recipient_privacy = self.privacy_repository.get(&recipient_id).await?;
        match recipient_privacy.dm_policy {
            DmPolicy::NoOne => return Err(SendDmMessageError::DmsClosed),
            DmPolicy::Friends if !are_friends => return Err(SendDmMessageError::NotFriends),
            _ => {}
        }

        if request.attachment_ids.len() > 10 {
            return Err(SendDmMessageError::InvalidAttachments);
        }
        let content = if request.attachment_ids.is_empty() {
            MessageContent::new(request.content)?
        } else {
            MessageContent::new_allow_empty(request.content)?
        };
        let now = Utc::now();
        let mut message = DmMessage::create(sender_id, recipient_id, content, now);
        let request_id = request.ctx.request_id.clone();
        let events = message.pull_events();

        let claim_token = uuid::Uuid::new_v4();
        let attachments = if request.attachment_ids.is_empty() {
            Vec::new()
        } else {
            match self
                .attachment_claims
                .claim(&request.attachment_ids, &sender_id, claim_token)
                .await
            {
                Ok(links) => links,
                Err(AttachmentClaimError::Invalid) => {
                    return Err(SendDmMessageError::InvalidAttachments);
                }
                Err(AttachmentClaimError::Unavailable(e)) => {
                    if let Err(re) = self.attachment_claims.release(&request.attachment_ids, claim_token).await
                    {
                        tracing::warn!(error = %re, "Failed to release attachments after ambiguous claim failure");
                    }
                    return Err(SendDmMessageError::AttachmentsUnavailable(e));
                }
            }
        };
        let persisted = async {
            let snapshots: Vec<AttachmentSnapshot> = attachments.iter().map(AttachmentSnapshot::from).collect();
        let events: Vec<MessagingEvent> = events.into_iter().map(|e| MessagingEvent::from(e.with_attachments(&snapshots))).collect();
        let mut tx = self.unit_of_work.begin().await?;
            tx.insert_message(&message, &attachments).await?;
            tx.save_events(&events, request_id.as_deref()).await?;
            tx.commit().await
        }
        .await;
        if let Err(e) = persisted {
            if !request.attachment_ids.is_empty() {
                match self.dm_repository.linked_attachment_subset(&request.attachment_ids).await {
                    Ok(linked) if linked.is_empty() => {
                        if let Err(re) = self
                            .attachment_claims
                            .release(&request.attachment_ids, claim_token)
                            .await
                        {
                            tracing::warn!(error = %re, "Failed to release attachment claims after send failure");
                        }
                    }
                    Ok(_) => {
                        tracing::warn!("Attachment links present after ambiguous commit failure; retaining claims");
                    }
                    Err(le) => {
                        tracing::warn!(error = %le, "Link reconciliation failed after commit failure; retaining claims");
                    }
                }
            }
            return Err(e.into());
        }

        let mut response = SendDmMessageResponse::from(&message);
        response.attachments = attachments;
        Ok(response)
    }
}
