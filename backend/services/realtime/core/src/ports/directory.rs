use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[async_trait]
pub trait RecipientDirectory: Send + Sync {
    async fn server_members(&self, server_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
    async fn user_name(&self, user_id: Uuid) -> Result<Option<String>, RepositoryError>;
    async fn user_name_versioned(&self, user_id: Uuid) -> Result<Option<(String, DateTime<Utc>)>, RepositoryError>;
    async fn rename_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
    async fn interested_in(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
}
