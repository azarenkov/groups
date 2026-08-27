use async_trait::async_trait;

use crate::{friendship::status::FriendshipStatus, friendship::view::FriendshipView, friendship::Friendship};
use domain::{shared::repository::error::RepositoryError, user::id::UserId};

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
    ) -> Result<Vec<FriendshipView>, RepositoryError>;
}
