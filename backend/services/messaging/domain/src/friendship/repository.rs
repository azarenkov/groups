use domain::pagination::KeysetCursor;
use crate::friendship::block::BlockRelations;
use crate::friendship::block::{BlockRelation, BlockedUserView};
use async_trait::async_trait;

use crate::{friendship::status::FriendshipStatus, friendship::view::FriendshipView, friendship::Friendship};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[async_trait]
pub trait FriendshipRepository {
    async fn find_pair(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<Option<Friendship>, RepositoryError>;

    async fn list_for_user(
        &self,
        user_id: &UserId,
        status: Option<FriendshipStatus>,
        cursor: Option<&KeysetCursor>,
        limit: i64,
    ) -> Result<Vec<FriendshipView>, RepositoryError>;

    async fn list_statuses_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<(UserId, FriendshipStatus)>, RepositoryError>;

    async fn block_relation(
        &self,
        viewer: &UserId,
        other: &UserId,
    ) -> Result<BlockRelation, RepositoryError>;

    async fn list_blocked(&self, blocker: &UserId) -> Result<Vec<BlockedUserView>, RepositoryError>;
    async fn blockers_among(
        &self,
        viewer: &UserId,
        candidates: &[UserId],
    ) -> Result<Vec<UserId>, RepositoryError>;
    async fn blocked_among(
        &self,
        blocker: &UserId,
        candidates: &[UserId],
    ) -> Result<Vec<UserId>, RepositoryError>;
    async fn block_relations_among(
        &self,
        user: &UserId,
        candidates: &[UserId],
    ) -> Result<BlockRelations, RepositoryError>;
}
