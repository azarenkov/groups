use async_trait::async_trait;
use domain::{server::id::ServerId, user::id::UserId};

#[derive(Debug, thiserror::Error)]
pub enum ServerOwnerLookupError {
    #[error("Owner lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait ServerOwnerLookup {
    async fn find_owner(
        &self,
        server_id: &ServerId,
    ) -> Result<Option<UserId>, ServerOwnerLookupError>;
}
