use std::sync::Arc;

use messaging_domain::{channel::id::ChannelId, channel::repository::ChannelRepository, channel::visibility::ChannelVisibility, message::repository::MessageRepository, server::id::ServerId, server::repository::ServerRepository};
use domain::{pagination::PaginatedResult, user::id::UserId};
use ports::repository::RepositoryError;

use crate::dto::{
    requests::channel::search_channel_messages::SearchChannelMessagesRequest,
    responses::channel::channel_message::ChannelMessageDto,
};

#[derive(Debug, thiserror::Error)]
pub enum SearchChannelMessagesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the server")]
    NotServerMember,
    #[error("Not a member of the channel")]
    NotChannelMember,
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Channel is not a text channel")]
    NotTextChannel,
    #[error("Search query cannot be empty")]
    EmptyQuery,
}

pub struct SearchChannelMessages<M, C, S>
where
    M: MessageRepository,
    C: ChannelRepository,
    S: ServerRepository,
{
    message_repository: Arc<M>,
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
}

impl<M, C, S> SearchChannelMessages<M, C, S>
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
        request: SearchChannelMessagesRequest,
        requester_id: UserId,
    ) -> Result<PaginatedResult<ChannelMessageDto>, SearchChannelMessagesError> {
        let trimmed = request.query.trim().to_string();
        if trimmed.is_empty() {
            return Err(SearchChannelMessagesError::EmptyQuery);
        }

        let server_id = ServerId::new(request.server_id);
        let channel_id = ChannelId::new(request.channel_id);

        let is_member = self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?;
        if !is_member {
            return Err(SearchChannelMessagesError::NotServerMember);
        }

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => SearchChannelMessagesError::ChannelNotFound,
                other => SearchChannelMessagesError::Repository(other),
            })?;
        if channel.server_id() != &server_id {
            return Err(SearchChannelMessagesError::ChannelNotFound);
        }

        if !channel.supports_text() {
            return Err(SearchChannelMessagesError::NotTextChannel);
        }

        if channel.visibility() == ChannelVisibility::Private {
            let is_text_member = self
                .channel_repository
                .is_text_channel_member(&channel_id, &requester_id)
                .await?;
            if !is_text_member {
                return Err(SearchChannelMessagesError::NotChannelMember);
            }
        }

        let result = self
            .message_repository
            .search_messages_in_channel(&channel_id, &trimmed, request.pagination)
            .await?;

        let items = result
            .items
            .into_iter()
            .map(ChannelMessageDto::from)
            .collect();
        Ok(PaginatedResult::new(items, result.total, result.pagination))
    }
}
