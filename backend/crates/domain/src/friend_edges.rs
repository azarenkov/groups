use async_trait::async_trait;

use crate::{shared::repository::error::RepositoryError, user::id::UserId};

#[async_trait]
pub trait FriendEdgeRepository {
    async fn are_friends(&self, a: &UserId, b: &UserId) -> Result<bool, RepositoryError>;
    async fn add_edge(&self, a: &UserId, b: &UserId) -> Result<(), RepositoryError>;
    async fn remove_edge(&self, a: &UserId, b: &UserId) -> Result<(), RepositoryError>;
}
