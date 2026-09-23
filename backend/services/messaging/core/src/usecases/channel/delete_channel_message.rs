use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::id::ChannelId, channel::repository::ChannelRepository, message::MessageError, message::id::MessageId, message::repository::MessageRepository, community::id::CommunityId, community::policy::PolicyError, community::unit_of_work::CommunityUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::dto::requests::channel::delete_channel_message::DeleteChannelMessageRequest;

#[derive(Debug, thiserror::Error)]
pub enum DeleteChannelMessageError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message domain error: {0}")]
    Domain(#[from] MessageError),
    #[error("Not a member of the community")]
    NotCommunityMember,
    #[error("Message not found")]
    MessageNotFound,
}

impl From<PolicyError> for DeleteChannelMessageError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => DeleteChannelMessageError::NotCommunityMember,
            _ => DeleteChannelMessageError::Domain(MessageError::NotAuthor),
        }
    }
}

pub struct DeleteChannelMessage<M, C, W>
where
    M: MessageRepository,
    C: ChannelRepository,
    W: CommunityUnitOfWork,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    unit_of_work: Arc<W>,
}

impl<M, C, W> DeleteChannelMessage<M, C, W>
where
    M: MessageRepository,
    C: ChannelRepository,
    W: CommunityUnitOfWork,
{
    pub fn new(message_repository: Arc<M>, channel_repository: Arc<C>, unit_of_work: Arc<W>) -> Self {
        Self {
            message_repository,
            channel_repository,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        request: DeleteChannelMessageRequest,
        deleter_id: UserId,
    ) -> Result<(), DeleteChannelMessageError> {
        let community_id = CommunityId::new(request.community_id);
        let channel_id = ChannelId::new(request.channel_id);
        let message_id = MessageId::new(request.message_id);

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

        if channel.community_id() != &community_id {
            return Err(DeleteChannelMessageError::MessageNotFound);
        }

        let recipient_ids = self
            .channel_repository
            .list_text_channel_member_ids(&channel_id)
            .await?;

        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = tx.load_community().await?;
        community.ensure_can_delete_message(&deleter_id, message.author_id())?;
        let is_owner = message.author_id() != &deleter_id;

        let now = Utc::now();
        message.mark_deleted(deleter_id, is_owner, now)?;

        let request_id = request.ctx.request_id.clone();
        let events = message.pull_events();
        let events: Vec<MessagingEvent> = events.into_iter().map(|e| MessagingEvent::from(e.with_recipients(&recipient_ids))).collect();

        tx.delete_message(&message_id).await.map_err(|e| match e {
            RepositoryError::NotFound => DeleteChannelMessageError::MessageNotFound,
            other => DeleteChannelMessageError::Repository(other),
        })?;
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(())
    }
}
