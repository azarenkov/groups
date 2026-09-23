use std::sync::Arc;

use messaging_domain::{channel::id::ChannelId, channel::repository::ChannelRepository, friendship::repository::FriendshipRepository, community::id::CommunityId, community::repository::CommunityRepository};
use domain::{pagination::PaginatedResult, user::id::UserId};
use ports::repository::RepositoryError;

use crate::dto::{
    requests::channel::list_channel_participants::ListChannelParticipantsRequest,
    responses::channel::channel_participant::ChannelParticipantDto,
};

#[derive(Debug, thiserror::Error)]
pub enum ListChannelParticipantsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the community")]
    NotMember,
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Not a member of the channel")]
    NotChannelMember,
}

pub struct ListChannelParticipants<C, S, F>
where
    C: ChannelRepository,
    S: CommunityRepository,
    F: FriendshipRepository,
{
    channel_repository: Arc<C>,
    community_repository: Arc<S>,
    friendship_repository: Arc<F>,
}

impl<C, S, F> ListChannelParticipants<C, S, F>
where
    C: ChannelRepository,
    S: CommunityRepository,
    F: FriendshipRepository,
{
    pub fn new(
        channel_repository: Arc<C>,
        community_repository: Arc<S>,
        friendship_repository: Arc<F>,
    ) -> Self {
        Self {
            channel_repository,
            community_repository,
            friendship_repository,
        }
    }

    pub async fn execute(
        &self,
        request: ListChannelParticipantsRequest,
    ) -> Result<PaginatedResult<ChannelParticipantDto>, ListChannelParticipantsError> {
        let community_id = CommunityId::new(request.community_id);
        let channel_id = ChannelId::new(request.channel_id);
        let requester_id = UserId::new(request.requester_id);

        let is_member = self
            .community_repository
            .is_community_member(&community_id, &requester_id)
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
        if channel.community_id() != &community_id {
            return Err(ListChannelParticipantsError::ChannelNotFound);
        }

        if channel.has_explicit_text_membership() {
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
            .list_channel_participants(&community_id, &channel_id, request.pagination)
            .await?;

        let mut items: Vec<ChannelParticipantDto> = result
            .items
            .into_iter()
            .map(ChannelParticipantDto::from)
            .collect();
        let ids: Vec<UserId> = items.iter().map(|i| UserId::new(i.id)).collect();
        let hidden = self
            .friendship_repository
            .blockers_among(&requester_id, &ids)
            .await?;
        for item in &mut items {
            if hidden.contains(&UserId::new(item.id)) {
                item.bio.clear();
            }
        }

        Ok(PaginatedResult::new(items, result.total, result.pagination))
    }
}
