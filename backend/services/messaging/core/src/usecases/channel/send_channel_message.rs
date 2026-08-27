use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::ChannelError, channel::id::ChannelId, channel::repository::ChannelRepository, message::Message, message::MessageError, message::content::MessageContent, message::repository::MessageRepository, message::unit_of_work::MessageUnitOfWork, server::id::ServerId, server::repository::ServerRepository};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};
use ports::repository::RepositoryError;

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
    #[error("Attachment service unavailable: {0}")]
    AttachmentsUnavailable(String),
}

pub struct SendChannelMessage<M, C, S, W, A, R>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
    W: MessageUnitOfWork,
    A: crate::ports::attachment_claims::AttachmentClaims,
    R: crate::ports::mention_resolver::MentionResolver,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
    unit_of_work: Arc<W>,
    attachment_claims: Arc<A>,
    mention_resolver: Arc<R>,
}

impl<M, C, S, W, A, R> SendChannelMessage<M, C, S, W, A, R>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
    W: MessageUnitOfWork,
    A: crate::ports::attachment_claims::AttachmentClaims,
    R: crate::ports::mention_resolver::MentionResolver,
{
    pub fn new(
        message_repository: Arc<M>,
        channel_repository: Arc<C>,
        server_repository: Arc<S>,
        unit_of_work: Arc<W>,
        attachment_claims: Arc<A>,
        mention_resolver: Arc<R>,
    ) -> Self {
        Self {
            message_repository,
            channel_repository,
            server_repository,
            unit_of_work,
            attachment_claims,
            mention_resolver,
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

        let mentions = crate::usecases::channel::mentions::resolve_mentions(
            message.content().as_str(),
            &channel_id,
            &recipient_ids,
            author_id.uuid(),
            self.channel_repository.as_ref(),
            self.mention_resolver.as_ref(),
        )
        .await?;

        let request_id = request.ctx.request_id.clone();
        let events = message.pull_events();

        let claim_token = uuid::Uuid::new_v4();
        let attachments = if request.attachment_ids.is_empty() {
            Vec::new()
        } else {
            match self
                .attachment_claims
                .claim(&request.attachment_ids, &author_id, claim_token)
                .await
            {
                Ok(links) => links,
                Err(crate::ports::attachment_claims::AttachmentClaimError::Invalid) => {
                    return Err(SendChannelMessageError::InvalidAttachments);
                }
                Err(crate::ports::attachment_claims::AttachmentClaimError::Unavailable(e)) => {
                    if let Err(re) = self.attachment_claims.release(&request.attachment_ids, claim_token).await
                    {
                        tracing::warn!(error = %re, "Failed to release attachments after ambiguous claim failure");
                    }
                    return Err(SendChannelMessageError::AttachmentsUnavailable(e));
                }
            }
        };

        let persisted = async {
            let snapshots: Vec<AttachmentSnapshot> = attachments.iter().map(AttachmentSnapshot::from).collect();
        let mention_ids: Vec<UserId> = mentions.iter().copied().map(UserId::new).collect();
        let events: Vec<MessagingEvent> = events.into_iter().map(|e| MessagingEvent::from(e.with_recipients(&recipient_ids).with_mentions(&mention_ids).with_attachments(&snapshots))).collect();
        let mut tx = self.unit_of_work.begin().await?;
            tx.insert_message(&message, &attachments, &mentions).await?;
            tx.save_events(&events, request_id.as_deref()).await?;
            tx.commit().await
        }
        .await;
        if let Err(e) = persisted {
            if !request.attachment_ids.is_empty() {
                match self.message_repository.linked_attachment_subset(&request.attachment_ids).await {
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

        let mut response = SendChannelMessageResponse::from(&message);
        response.attachments = attachments;
        Ok(response)
    }
}
