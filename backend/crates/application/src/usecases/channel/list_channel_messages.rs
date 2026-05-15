use std::sync::Arc;

use domain::{
    channel::{id::ChannelId, repository::ChannelRepository, visibility::ChannelVisibility},
    message::{id::MessageId, repository::MessageRepository},
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::dto::{
    requests::channel::list_channel_messages::ListChannelMessagesRequest,
    responses::channel::channel_message::ChannelMessageDto,
};

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, thiserror::Error)]
pub enum ListChannelMessagesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the server")]
    NotServerMember,
    #[error("Not a member of the channel")]
    NotChannelMember,
    #[error("Channel not found")]
    ChannelNotFound,
}

pub struct ListChannelMessages<M, C, S>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
}

impl<M, C, S> ListChannelMessages<M, C, S>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
{
    pub fn new(
        message_repository: Arc<M>,
        channel_repository: Arc<C>,
        server_repository: Arc<S>,
    ) -> Self {
        Self {
            message_repository,
            channel_repository,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        request: ListChannelMessagesRequest,
        requester_id: UserId,
    ) -> Result<Vec<ChannelMessageDto>, ListChannelMessagesError> {
        let server_id = ServerId::new(request.server_id);
        let channel_id = ChannelId::new(request.channel_id);

        let is_member = self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?;
        if !is_member {
            return Err(ListChannelMessagesError::NotServerMember);
        }

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => ListChannelMessagesError::ChannelNotFound,
                other => ListChannelMessagesError::Repository(other),
            })?;
        if channel.server_id() != &server_id {
            return Err(ListChannelMessagesError::ChannelNotFound);
        }

        if channel.visibility() == ChannelVisibility::Private
            && !channel.is_text_member(&requester_id)
        {
            return Err(ListChannelMessagesError::NotChannelMember);
        }

        let limit = clamp_limit(request.limit);
        let before = request.before.map(MessageId::new);
        let after = request.after.map(MessageId::new);

        let messages = self
            .message_repository
            .list_messages_in_channel(&channel_id, before.as_ref(), after.as_ref(), limit)
            .await?;

        Ok(messages.into_iter().map(ChannelMessageDto::from).collect())
    }
}

fn clamp_limit(value: i64) -> i64 {
    if value <= 0 {
        DEFAULT_LIMIT
    } else if value > MAX_LIMIT {
        MAX_LIMIT
    } else {
        value
    }
}
