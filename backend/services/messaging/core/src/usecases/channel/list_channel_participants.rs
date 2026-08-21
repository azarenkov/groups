use std::sync::Arc;

use domain::{
    channel::{id::ChannelId, repository::ChannelRepository, visibility::ChannelVisibility},
    server::{id::ServerId, repository::ServerRepository},
    shared::{pagination::PaginatedResult, repository::error::RepositoryError},
    user::id::UserId,
};

use crate::dto::{
    requests::channel::list_channel_participants::ListChannelParticipantsRequest,
    responses::channel::channel_participant::ChannelParticipantDto,
};

#[derive(Debug, thiserror::Error)]
pub enum ListChannelParticipantsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the server")]
    NotMember,
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Not a member of the channel")]
    NotChannelMember,
}

pub struct ListChannelParticipants<C, S>
where
    C: ChannelRepository,
    S: ServerRepository,
{
    channel_repository: Arc<C>,
    server_repository: Arc<S>,
}

impl<C, S> ListChannelParticipants<C, S>
where
    C: ChannelRepository,
    S: ServerRepository,
{
    pub fn new(channel_repository: Arc<C>, server_repository: Arc<S>) -> Self {
        Self {
            channel_repository,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        request: ListChannelParticipantsRequest,
    ) -> Result<PaginatedResult<ChannelParticipantDto>, ListChannelParticipantsError> {
        let server_id = ServerId::new(request.server_id);
        let channel_id = ChannelId::new(request.channel_id);
        let requester_id = UserId::new(request.requester_id);

        let is_member = self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?;
        if !is_member {
            return Err(ListChannelParticipantsError::NotMember);
        }

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => ListChannelParticipantsError::ChannelNotFound,
                other => ListChannelParticipantsError::Repository(other),
            })?;
        if channel.server_id() != &server_id {
            return Err(ListChannelParticipantsError::ChannelNotFound);
        }

        if channel.supports_text() && channel.visibility() == ChannelVisibility::Private {
            let is_text_member = self
                .channel_repository
                .is_text_channel_member(&channel_id, &requester_id)
                .await?;
            if !is_text_member {
                return Err(ListChannelParticipantsError::NotChannelMember);
            }
        }

        let result = self
            .channel_repository
            .list_channel_participants(&server_id, &channel_id, request.pagination)
            .await?;

        let items = result
            .items
            .into_iter()
            .map(ChannelParticipantDto::from)
            .collect();

        Ok(PaginatedResult::new(items, result.total, result.pagination))
    }
}
