use async_trait::async_trait;
use ports::repository::RepositoryError;

#[async_trait]
pub trait StorageReleases {
    async fn release(&self, storage_keys: &[String]) -> Result<(), RepositoryError>;
}
