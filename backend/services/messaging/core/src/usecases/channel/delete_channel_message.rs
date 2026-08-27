use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{channel::id::ChannelId, channel::repository::ChannelRepository, message::MessageError, message::id::MessageId, message::repository::MessageRepository, server::id::ServerId, server::policy::ModerationPolicy, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};

use crate::dto::requests::channel::delete_channel_message::DeleteChannelMessageRequest;

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

impl From<PolicyError> for DeleteChannelMessageError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => DeleteChannelMessageError::NotServerMember,
            _ => DeleteChannelMessageError::Domain(MessageError::NotAuthor),
        }
    }
}

pub struct DeleteChannelMessage<M, C, W>
where
    M: MessageRepository,
    C: ChannelRepository,
    W: ServerUnitOfWork,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    unit_of_work: Arc<W>,
}

impl<M, C, W> DeleteChannelMessage<M, C, W>
where
    M: MessageRepository,
    C: ChannelRepository,
    W: ServerUnitOfWork,
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
        let server_id = ServerId::new(request.server_id);
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

        if channel.server_id() != &server_id {
            return Err(DeleteChannelMessageError::MessageNotFound);
        }

        let recipient_ids = self
            .channel_repository
            .list_text_channel_member_ids(&channel_id)
            .await?;

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        ModerationPolicy::delete_message(&snapshot, &deleter_id, message.author_id())?;
        let is_owner = message.author_id() != &deleter_id;

        let now = Utc::now();
        message.mark_deleted(deleter_id, is_owner, now)?;

        let request_id = request.ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = message
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        crate::usecases::channel::recipients::inject_recipient_ids(&mut records, &recipient_ids);

        tx.delete_message(&message_id).await.map_err(|e| match e {
            RepositoryError::NotFound => DeleteChannelMessageError::MessageNotFound,
            other => DeleteChannelMessageError::Repository(other),
        })?;
        tx.save_outbox(&records).await?;
        tx.commit().await?;

        Ok(())
    }
}
