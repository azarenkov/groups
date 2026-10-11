use std::collections::HashSet;
use std::sync::Arc;

use messaging_domain::badge::attachments_per_message;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, privacy::ContactPolicy, privacy::PrivacyRepository, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, dm_message::DmMessageError, dm_message::id::DmMessageId, dm_message::repository::DmMessageRepository, dm_message::unit_of_work::DmUnitOfWork, message::MessageError, message::content::MessageContent};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};
use ports::repository::RepositoryError;
use ports::user_profile::UserProfileRepository;
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
    #[error("Message not found")]
    NotFound,
    #[error("Not friends with this user")]
    NotFriends,
    #[error("This user is blocked")]
    Blocked,
    #[error("This user does not accept direct messages")]
    DmsClosed,
    #[error("This user is unavailable")]
    RecipientInactive,
    #[error("Invalid attachments")]
    InvalidAttachments,
    #[error("Attachment service unavailable: {0}")]
    AttachmentsUnavailable(String),
}

pub struct EditDmMessage<M, W, A, F, P, U>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
    A: AttachmentClaims,
    F: FriendshipRepository,
    P: PrivacyRepository,
    U: UserProfileRepository,
{
    dm_repository: Arc<M>,
    unit_of_work: Arc<W>,
    attachment_claims: Arc<A>,
    friendship_repository: Arc<F>,
    privacy_repository: Arc<P>,
    user_profile_repository: Arc<U>,
    plans: Arc<dyn messaging_domain::badge::BadgeRepository>,
}

impl<M, W, A, F, P, U> EditDmMessage<M, W, A, F, P, U>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
    A: AttachmentClaims,
    F: FriendshipRepository,
    P: PrivacyRepository,
    U: UserProfileRepository,
{
    pub fn new(
        dm_repository: Arc<M>,
        unit_of_work: Arc<W>,
        attachment_claims: Arc<A>,
        friendship_repository: Arc<F>,
        privacy_repository: Arc<P>,
        user_profile_repository: Arc<U>,
        plans: Arc<dyn messaging_domain::badge::BadgeRepository>,
    ) -> Self {
        Self {
            dm_repository,
            unit_of_work,
            attachment_claims,
            friendship_repository,
            privacy_repository,
            user_profile_repository,
            plans,
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

        if !self
            .user_profile_repository
            .is_active(message.recipient_id())
            .await?
        {
            return Err(EditDmMessageError::RecipientInactive);
        }

        if self
            .friendship_repository
            .block_relation(message.sender_id(), message.recipient_id())
            .await?
            .any()
        {
            return Err(EditDmMessageError::Blocked);
        }
        let pair = self
            .friendship_repository
            .find_pair(message.sender_id(), message.recipient_id())
            .await?;
        let are_friends = matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted);
        match self
            .privacy_repository
            .get(message.recipient_id())
            .await?
            .dm_policy
        {
            ContactPolicy::NoOne => return Err(EditDmMessageError::DmsClosed),
            ContactPolicy::Friends if !are_friends => return Err(EditDmMessageError::NotFriends),
            _ => {}
        }

        let current: Vec<domain::attachment::AttachmentLink> = self
            .dm_repository
            .list_attachment_links_for_dm_messages(&[message_id.uuid()])
            .await?
            .into_iter()
            .map(|(_, link)| link)
            .collect();
        if current.iter().any(messaging_domain::message::voice::is_rich_media) {
            return Err(EditDmMessageError::InvalidAttachments);
        }
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
        let tier = self.plans.tier_of(&editor_id, Utc::now()).await?;
        if !add_ids.is_empty() && remaining > attachments_per_message(&tier) {
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
        let events = message.pull_events();
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
        if added_attachments.iter().any(messaging_domain::message::voice::is_rich_media)
            || !messaging_domain::message::voice::voice_message_is_valid(
                message.content().as_str(),
                &final_attachments,
            )
        {
            if !added_attachments.is_empty()
                && let Err(re) = self
                    .attachment_claims
                    .release(&request.add_attachment_ids, claim_token)
                    .await
            {
                tracing::warn!(error = %re, "Failed to release attachments of an invalid voice message");
            }
            return Err(EditDmMessageError::InvalidAttachments);
        }
        let persisted = async {
            let snapshots: Vec<AttachmentSnapshot> = final_attachments.iter().map(AttachmentSnapshot::from).collect();
            let mut tx = self.unit_of_work.begin().await?;
            tx.lock_pair(message.sender_id(), message.recipient_id()).await?;
            if tx
                .block_relation(message.sender_id(), message.recipient_id())
                .await?
                .any()
            {
                return Err(EditDmMessageError::Blocked);
            }
            let hidden_for = tx.users_hiding(message.id().uuid()).await?;
            let events: Vec<MessagingEvent> = events
                .into_iter()
                .map(|e| MessagingEvent::from(e.with_attachments(&snapshots).with_hidden_for(&hidden_for)))
                .collect();
            tx.update_message(&message, &added_attachments, &request.remove_attachment_ids)
                .await?;
            tx.save_events(&events, request_id.as_deref()).await?;
            tx.commit().await?;
            Ok::<(), EditDmMessageError>(())
        }
        .await;
        if let Err(e) = persisted {
            if !request.add_attachment_ids.is_empty() {
                match self.dm_repository.linked_attachment_subset(&request.add_attachment_ids).await {
                    Ok(linked) if linked.is_empty() => {
                        if let Err(re) = self
                            .attachment_claims
                            .release(&request.add_attachment_ids, claim_token)
                            .await
                        {
                            tracing::warn!(error = %re, "Failed to release attachment claims after edit failure");
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
            return Err(match e {
                EditDmMessageError::Repository(RepositoryError::InvalidData(_)) => {
                    EditDmMessageError::InvalidAttachments
                }
                other => other,
            });
        }
        Ok(())
    }
}
