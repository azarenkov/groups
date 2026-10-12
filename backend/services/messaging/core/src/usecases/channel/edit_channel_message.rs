use std::collections::HashSet;
use std::sync::Arc;

use messaging_domain::badge::attachments_per_message;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::id::ChannelId, channel::repository::ChannelRepository, message::MessageError, message::content::MessageContent, message::id::MessageId, message::repository::MessageRepository, message::unit_of_work::MessageUnitOfWork, community::id::CommunityId, community::repository::CommunityRepository};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::dto::requests::channel::edit_channel_message::EditChannelMessageRequest;

#[derive(Debug, thiserror::Error)]
pub enum EditChannelMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message domain error: {0}")]
    Domain(#[from] MessageError),
    #[error("Not a member of the community")]
    NotCommunityMember,
    #[error("Message not found")]
    MessageNotFound,
    #[error("Invalid attachments")]
    InvalidAttachments,
    #[error("Attachment service unavailable: {0}")]
    AttachmentsUnavailable(String),
}

pub struct EditChannelMessage<M, C, S, W, A, R>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: CommunityRepository,
    W: MessageUnitOfWork,
    A: crate::ports::attachment_claims::AttachmentClaims,
    R: crate::ports::mention_resolver::MentionResolver,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    community_repository: Arc<S>,
    unit_of_work: Arc<W>,
    attachment_claims: Arc<A>,
    mention_resolver: Arc<R>,
    plans: Arc<dyn messaging_domain::badge::BadgeRepository>,
}

impl<M, C, S, W, A, R> EditChannelMessage<M, C, S, W, A, R>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: CommunityRepository,
    W: MessageUnitOfWork,
    A: crate::ports::attachment_claims::AttachmentClaims,
    R: crate::ports::mention_resolver::MentionResolver,
{
    pub fn new(
        message_repository: Arc<M>,
        channel_repository: Arc<C>,
        community_repository: Arc<S>,
        unit_of_work: Arc<W>,
        attachment_claims: Arc<A>,
        mention_resolver: Arc<R>,
        plans: Arc<dyn messaging_domain::badge::BadgeRepository>,
    ) -> Self {
        Self {
            message_repository,
            channel_repository,
            community_repository,
            unit_of_work,
            attachment_claims,
            mention_resolver,
            plans,
        }
    }

    pub async fn execute(
        &self,
        request: EditChannelMessageRequest,
        editor_id: UserId,
    ) -> Result<(), EditChannelMessageError> {
        let community_id = CommunityId::new(request.community_id);
        let channel_id = ChannelId::new(request.channel_id);
        let message_id = MessageId::new(request.message_id);

        let is_member = self
            .community_repository
            .is_community_member(&community_id, &editor_id)
            .await?;
        if !is_member {
            return Err(EditChannelMessageError::NotCommunityMember);
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

        if channel.community_id() != &community_id {
            return Err(EditChannelMessageError::MessageNotFound);
        }

        let current: Vec<domain::attachment::AttachmentLink> = self
            .message_repository
            .list_attachment_links_for_messages(&[message_id.uuid()])
            .await?
            .into_iter()
            .map(|(_, link)| link)
            .collect();
        if current.iter().any(messaging_domain::message::voice::is_rich_media) {
            return Err(EditChannelMessageError::InvalidAttachments);
        }
        let current_ids: HashSet<Uuid> = current.iter().map(|a| a.id).collect();
        let remove_ids: HashSet<Uuid> = request.remove_attachment_ids.iter().copied().collect();
        if !remove_ids.iter().all(|id| current_ids.contains(id)) {
            return Err(EditChannelMessageError::InvalidAttachments);
        }
        let add_ids: HashSet<Uuid> = request.add_attachment_ids.iter().copied().collect();
        if add_ids.iter().any(|id| current_ids.contains(id)) {
            return Err(EditChannelMessageError::InvalidAttachments);
        }
        let remaining = current_ids.len() - remove_ids.len() + add_ids.len();
        let tier = self.plans.tier_of(&editor_id, Utc::now()).await?;
        if !add_ids.is_empty() && remaining > attachments_per_message(&tier) {
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

        let mut mentions = crate::usecases::channel::mentions::resolve_mentions(
            message.content().as_str(),
            &channel_id,
            &recipient_ids,
            message.author_id().uuid(),
            self.channel_repository.as_ref(),
            self.mention_resolver.as_ref(),
        )
        .await?;
        if let Some(replied) = self.message_repository.find_reply_author(message.id()).await? {
            if replied != *message.author_id()
                && recipient_ids.contains(&replied)
                && !mentions.contains(&replied.uuid())
            {
                mentions.push(replied.uuid());
            }
        }

        let hidden_for = self
            .message_repository
            .list_users_hiding(message.id().uuid())
            .await?;
        let (event_recipients, skipped_recipients): (Vec<_>, Vec<_>) =
            recipient_ids.iter().copied().partition(|id| !hidden_for.contains(id));
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
                Err(crate::ports::attachment_claims::AttachmentClaimError::Invalid) => {
                    return Err(EditChannelMessageError::InvalidAttachments);
                }
                Err(crate::ports::attachment_claims::AttachmentClaimError::Unavailable(e)) => {
                    if let Err(re) = self
                        .attachment_claims
                        .release(&request.add_attachment_ids, claim_token)
                        .await
                    {
                        tracing::warn!(error = %re, "Failed to release attachments after ambiguous claim failure");
                    }
                    return Err(EditChannelMessageError::AttachmentsUnavailable(e));
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
            return Err(EditChannelMessageError::InvalidAttachments);
        }

        let persisted = async {
            let snapshots: Vec<AttachmentSnapshot> = final_attachments.iter().map(AttachmentSnapshot::from).collect();
        let events: Vec<MessagingEvent> = events.into_iter().map(|e| MessagingEvent::from(e.with_recipients(&event_recipients).with_skipped(&skipped_recipients).with_attachments(&snapshots))).collect();
        let mut tx = self.unit_of_work.begin().await?;
            tx.update_message(
                &message,
                &added_attachments,
                &request.remove_attachment_ids,
                &mentions,
            )
            .await?;
            tx.save_events(&events, request_id.as_deref()).await?;
            tx.commit().await
        }
        .await;
        if let Err(e) = persisted {
            if !request.add_attachment_ids.is_empty() {
                match self.message_repository.linked_attachment_subset(&request.add_attachment_ids).await {
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
                RepositoryError::InvalidData(_) => EditChannelMessageError::InvalidAttachments,
                other => EditChannelMessageError::Repository(other),
            });
        }

        Ok(())
    }
}
