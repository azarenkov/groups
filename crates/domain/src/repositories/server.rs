use async_trait::async_trait;

use crate::{repositories::error::RepositoryError, value_objects::server::id::ServerId};

#[async_trait]
pub trait ServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError>;
}
