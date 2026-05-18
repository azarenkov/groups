use async_trait::async_trait;

use crate::{
    friendship::{status::FriendshipStatus, view::FriendshipView, Friendship},
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};

#[async_trait]
pub trait FriendshipRepository {
    async fn find_pair(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<Option<Friendship>, RepositoryError>;

    async fn save_request_with_events(
        &self,
        outgoing: &Friendship,
        incoming: &Friendship,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn accept_pair_with_events(
        &self,
        accepter: &UserId,
        sender: &UserId,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;

    async fn delete_pair_with_events(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;

    async fn decline_request_with_events(
        &self,
        decliner: &UserId,
        sender: &UserId,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;

    async fn list_for_user(
        &self,
        user_id: &UserId,
        status: Option<FriendshipStatus>,
    ) -> Result<Vec<FriendshipView>, RepositoryError>;
}
