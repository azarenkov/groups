use async_trait::async_trait;
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum UserStateLookupError {
    #[error("User state lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait UserStateLookup {
    async fn has_messaging_state(
        &self,
        user_id: &UserId,
    ) -> Result<bool, UserStateLookupError>;
}
