use std::sync::Arc;

use domain::{
    channel::{id::ChannelId, repository::ChannelRepository},
    shared::{pagination::PaginatedResult, repository::error::RepositoryError},
};

use crate::dto::{
    requests::channel::list_channel_participants::ListChannelParticipantsRequest,
    responses::channel::channel_participant::ChannelParticipantDto,
};

#[derive(Debug, thiserror::Error)]
pub enum ListChannelParticipantsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListChannelParticipants<T>
where
    T: ChannelRepository,
{
    channel_repository: Arc<T>,
}

impl<T> ListChannelParticipants<T>
where
    T: ChannelRepository,
{
    pub fn new(channel_repository: Arc<T>) -> Self {
        Self { channel_repository }
    }

    pub async fn execute(
        &self,
        request: ListChannelParticipantsRequest,
    ) -> Result<PaginatedResult<ChannelParticipantDto>, ListChannelParticipantsError> {
        let channel_id = ChannelId::new(request.channel_id);

        let result = self
            .channel_repository
            .list_channel_participants(&channel_id, request.pagination)
            .await?;

        let items = result
            .items
            .into_iter()
            .map(ChannelParticipantDto::from)
            .collect();

        Ok(PaginatedResult::new(items, result.total, result.pagination))
    }
}
