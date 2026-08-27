use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::events::MessagingEvent;
use crate::friendship::Friendship;

#[async_trait::async_trait]
pub trait FriendshipUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn FriendshipTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait FriendshipTransaction: Send {
    async fn insert_request_pair(
        &mut self,
        outgoing: &Friendship,
        incoming: &Friendship,
    ) -> Result<(), RepositoryError>;
    async fn accept_pair(&mut self, accepter: &UserId, sender: &UserId) -> Result<bool, RepositoryError>;
    async fn decline_request(&mut self, decliner: &UserId, sender: &UserId) -> Result<bool, RepositoryError>;
    async fn delete_pair(&mut self, user_id: &UserId, friend_id: &UserId) -> Result<bool, RepositoryError>;
    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
