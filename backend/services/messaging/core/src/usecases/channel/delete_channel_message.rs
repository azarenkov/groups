use std::sync::Arc;

use chrono::Utc;
use domain::{
    server::role::{PERM_MANAGE_MESSAGES, ServerRoleRepository, has_permission},
    channel::{id::ChannelId, repository::ChannelRepository},
    message::{MessageError, id::MessageId, repository::MessageRepository},
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

pub struct DeleteChannelMessage<M, C, S, R>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
    R: ServerRoleRepository,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<M, C, S, R> DeleteChannelMessage<M, C, S, R>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
    R: ServerRoleRepository,
{
    pub fn new(
        message_repository: Arc<M>,
        channel_repository: Arc<C>,
        server_repository: Arc<S>,
        role_repository: Arc<R>,
    ) -> Self {
        Self {
            message_repository,
            channel_repository,
            server_repository,
            role_repository,
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

        let is_member = self
            .server_repository
            .is_server_member(&server_id, &deleter_id)
            .await?;
        if !is_member {
            return Err(DeleteChannelMessageError::NotServerMember);
        }

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

        let inherent = channel.owner_id() == &deleter_id
            || message.author_id() == &deleter_id
            || {
                let server_owner = self
                    .server_repository
                    .find_server_owner(&server_id)
                    .await?;
                server_owner.as_ref() == Some(&deleter_id)
            };
        let is_owner = inherent
            || has_permission(
                self.role_repository
                    .user_permissions(&server_id, &deleter_id)
                    .await?,
                PERM_MANAGE_MESSAGES,
            );

        let now = Utc::now();
        message.mark_deleted(deleter_id, is_owner, now)?;

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

        let delegated = if inherent {
            None
        } else {
            Some((&server_id, &deleter_id))
        };
        if !self
            .message_repository
            .delete_message_with_events(&message_id, delegated, &records)
            .await?
        {
            return Err(DeleteChannelMessageError::Domain(
                domain::message::MessageError::NotAuthor,
            ));
        }

        Ok(())
    }
}
