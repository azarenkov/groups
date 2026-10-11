use std::sync::Arc;

use messaging_domain::badge::attachments_per_message;

use chrono::Utc;
use messaging_domain::{privacy::ContactPolicy, privacy::PrivacyRepository, events::MessagingEvent, dm_message::DmMessage, dm_message::repository::DmMessageRepository, dm_message::unit_of_work::DmUnitOfWork, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::MessageError, message::content::MessageContent, message::reply::ReplyPreview, dm_message::id::DmMessageId};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};
use ports::repository::RepositoryError;
use ports::user_profile::UserProfileRepository;

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
    #[error("This user is blocked")]
    Blocked,
    #[error("This user does not accept direct messages")]
    DmsClosed,
    #[error("Cannot send a direct message to yourself")]
    SelfDm,
    #[error("This user is unavailable")]
    RecipientInactive,
    #[error("Invalid attachments")]
    InvalidAttachments,
    #[error("Invalid sticker")]
    InvalidSticker,
    #[error("Attachment service unavailable: {0}")]
    AttachmentsUnavailable(String),
    #[error("The message being replied to is not in this conversation")]
    InvalidReply,
}

pub struct SendDmMessage<M, W, F, A, P, U>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
    F: FriendshipRepository,
    A: AttachmentClaims,
    P: PrivacyRepository,
    U: UserProfileRepository,
{
    dm_repository: Arc<M>,
    unit_of_work: Arc<W>,
    friendship_repository: Arc<F>,
    attachment_claims: Arc<A>,
    privacy_repository: Arc<P>,
    user_profile_repository: Arc<U>,
    plans: Arc<dyn messaging_domain::badge::BadgeRepository>,
}

impl<M, W, F, A, P, U> SendDmMessage<M, W, F, A, P, U>
where
    M: DmMessageRepository,
    W: DmUnitOfWork,
    F: FriendshipRepository,
    A: AttachmentClaims,
    P: PrivacyRepository,
    U: UserProfileRepository,
{
    pub fn new(
        dm_repository: Arc<M>,
        unit_of_work: Arc<W>,
        friendship_repository: Arc<F>,
        attachment_claims: Arc<A>,
        privacy_repository: Arc<P>,
        user_profile_repository: Arc<U>,
        plans: Arc<dyn messaging_domain::badge::BadgeRepository>,
    ) -> Self {
        Self {
            dm_repository,
            unit_of_work,
            friendship_repository,
            attachment_claims,
            privacy_repository,
            user_profile_repository,
            plans,
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
        if !self.user_profile_repository.is_active(&recipient_id).await? {
            return Err(SendDmMessageError::RecipientInactive);
        }
        if self
            .friendship_repository
            .block_relation(&sender_id, &recipient_id)
            .await?
            .any()
        {
            return Err(SendDmMessageError::Blocked);
        }
        let pair = self
            .friendship_repository
            .find_pair(&sender_id, &recipient_id)
            .await?;
        let are_friends = matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted);
        let recipient_privacy = self.privacy_repository.get(&recipient_id).await?;
        match recipient_privacy.dm_policy {
            ContactPolicy::NoOne => return Err(SendDmMessageError::DmsClosed),
            ContactPolicy::Friends if !are_friends => return Err(SendDmMessageError::NotFriends),
            _ => {}
        }

        let tier = self.plans.tier_of(&sender_id, Utc::now()).await?;
        if request.attachment_ids.len() > attachments_per_message(&tier) {
            return Err(SendDmMessageError::InvalidAttachments);
        }
        let forward = request.forward.clone();
        let sticker_id = if forward.is_some() {
            None
        } else {
            request.sticker_id
        };
        if sticker_id.is_some() && (!request.content.trim().is_empty() || !request.attachment_ids.is_empty()) {
            return Err(SendDmMessageError::InvalidSticker);
        }
        let sticker = forward.as_ref().and_then(|forward| forward.sticker.clone());
        let content = if let Some(forward) = &forward {
            if forward.content.trim().is_empty() && forward.attachments.is_empty() && forward.sticker.is_none() {
                return Err(MessageError::ContentEmpty.into());
            }
            MessageContent::new_allow_empty(forward.content.clone())?
        } else if sticker_id.is_some() {
            MessageContent::new_allow_empty(String::new())?
        } else if request.attachment_ids.is_empty() {
            MessageContent::new(request.content)?
        } else {
            MessageContent::new_allow_empty(request.content)?
        };
        let now = Utc::now();
        let mut message = DmMessage::create(sender_id, recipient_id, content, now);
        message = message.with_sticker(sticker.clone());
        if let Some(forward) = &forward {
            message = message.with_forward(forward.info.clone());
        }
        let mut reply_author_name = None;
        if let Some(reply_id) = request.reply_to_message_id {
            let target = self
                .dm_repository
                .find_view_between(&DmMessageId::new(reply_id), &sender_id, &recipient_id)
                .await
                .map_err(|e| match e {
                    RepositoryError::NotFound => SendDmMessageError::InvalidReply,
                    other => SendDmMessageError::Repository(other),
                })?;
            if target.call_status.is_some() || target.invite_community_id.is_some() {
                return Err(SendDmMessageError::InvalidReply);
            }
            let reply_links: Vec<domain::attachment::AttachmentLink> = self
                .dm_repository
                .list_attachment_links_for_dm_messages(&[reply_id])
                .await?
                .into_iter()
                .map(|(_, link)| link)
                .collect();
            let has_attachments = !reply_links.is_empty();
            let hidden_for = self.dm_repository.list_users_hiding(reply_id).await?;
            reply_author_name = Some(target.sender_name.clone());
            message = message.with_reply(
                ReplyPreview::new(reply_id, target.sender_id, &target.content, has_attachments)
                    .with_voice(messaging_domain::message::voice::voice_duration_of(&reply_links))
                    .with_video_note(messaging_domain::message::voice::video_note_of(&reply_links))
                    .with_sticker(target.sticker.clone())
                    .hidden_for(hidden_for),
            );
        }
        let request_id = request.ctx.request_id.clone();
        if let Some(sticker_id) = sticker_id {
            let claimed = match self.attachment_claims.lookup_sticker(sticker_id).await {
                Ok(Some(sticker)) => sticker,
                Ok(None) | Err(crate::ports::attachment_claims::AttachmentClaimError::Invalid) => {
                    return Err(SendDmMessageError::InvalidSticker);
                }
                Err(crate::ports::attachment_claims::AttachmentClaimError::Unavailable(e)) => {
                    return Err(SendDmMessageError::AttachmentsUnavailable(e));
                }
            };
            message = message.with_sticker(Some(claimed));
        }
        let claim_token = uuid::Uuid::new_v4();
        let attachments = if let Some(forward) = &forward {
            forward.attachments.clone()
        } else if request.attachment_ids.is_empty() {
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
        if !messaging_domain::message::voice::voice_message_is_valid(message.content().as_str(), &attachments) {
            if forward.is_none()
                && !request.attachment_ids.is_empty()
                && let Err(re) = self.attachment_claims.release(&request.attachment_ids, claim_token).await
            {
                tracing::warn!(error = %re, "Failed to release attachments of an invalid voice message");
            }
            return Err(SendDmMessageError::InvalidAttachments);
        }
        let persisted = async {
            let snapshots: Vec<AttachmentSnapshot> = attachments.iter().map(AttachmentSnapshot::from).collect();
            let mut tx = self.unit_of_work.begin().await?;
            tx.lock_pair(&sender_id, &recipient_id).await?;
            if tx.block_relation(&sender_id, &recipient_id).await?.any() {
                return Err(SendDmMessageError::Blocked);
            }
            message.restamp(Utc::now());
            let events: Vec<MessagingEvent> = message
                .pull_events()
                .into_iter()
                .map(|e| MessagingEvent::from(e.with_attachments(&snapshots)))
                .collect();
            tx.insert_message(&message, &attachments).await?;
            tx.save_events(&events, request_id.as_deref()).await?;
            tx.commit().await?;
            Ok::<(), SendDmMessageError>(())
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
            return Err(e);
        }

        let mut response = SendDmMessageResponse::from(&message);
        response.attachments = attachments;
        if let Some(reply) = response.reply.as_mut() {
            reply.author_name = reply_author_name;
        }
        Ok(response)
    }
}
