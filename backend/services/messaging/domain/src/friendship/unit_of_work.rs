use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::{events::MessagingEvent, friendship::pair::FriendshipPair};

#[async_trait::async_trait]
pub trait FriendshipUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn FriendshipTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait FriendshipTransaction: Send {
    async fn load_pair(&mut self, a: &UserId, b: &UserId) -> Result<Option<FriendshipPair>, RepositoryError>;
    async fn save_pair(&mut self, pair: &FriendshipPair) -> Result<(), RepositoryError>;
    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
