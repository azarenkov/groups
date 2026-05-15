use std::sync::Arc;

use domain::{
    channel::{id::ChannelId, repository::ChannelRepository},
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
