use async_trait::async_trait;
use uuid::Uuid;

use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[async_trait]
pub trait HiddenMessageRepository: Send + Sync {
    async fn hide_for_user(&self, user_id: &UserId, message_id: Uuid) -> Result<(), RepositoryError>;
}
