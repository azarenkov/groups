use domain::pagination::KeysetCursor;
use async_trait::async_trait;

use crate::{channel::Channel, community::Community, community::id::CommunityId, community::invite_link::InviteLink, community::invite_link::InviteToken};
use domain::{pagination::PaginatedResult, pagination::Pagination, user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct MutualCommunity {
    pub community_id: CommunityId,
    pub name: String,
    pub accent_color: Option<String>,
}

#[async_trait]
pub trait CommunityRepository {
    async fn community_exists(&self, community_id: &CommunityId) -> Result<bool, RepositoryError>;
    async fn find_communities_by_user_id(
        &self,
        user_id: &UserId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Community>, RepositoryError>;
    async fn find_users_by_community_id(
        &self,
        community_id: &CommunityId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<(UserId, UserName)>, RepositoryError>;
    async fn find_member_ids_by_community_id(
        &self,
        community_id: &CommunityId,
    ) -> Result<Vec<UserId>, RepositoryError>;
    async fn find_channels_by_community_id(
        &self,
        community_id: &CommunityId,
        pagination: Pagination,
    ) -> Result<PaginatedResult<Channel>, RepositoryError>;
    async fn find_community_owner(
        &self,
        community_id: &CommunityId,
    ) -> Result<Option<UserId>, RepositoryError>;
    async fn find_mutual_communities(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Vec<MutualCommunity>, RepositoryError>;

    async fn find_community_by_id(
        &self,
        community_id: &CommunityId,
    ) -> Result<Option<Community>, RepositoryError>;
    async fn is_community_member(
        &self,
        community_id: &CommunityId,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;
    async fn list_invite_links(
        &self,
        community_id: &CommunityId,
        cursor: Option<&KeysetCursor>,
        limit: i64,
    ) -> Result<Vec<InviteLink>, RepositoryError>;
    async fn find_invite_link(
        &self,
        token: &InviteToken,
    ) -> Result<Option<InviteLink>, RepositoryError>;
    async fn count_members(&self, community_id: &CommunityId) -> Result<i64, RepositoryError>;

}
