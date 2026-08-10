use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum FileStorageError {
    #[error("Storage error: {0}")]
    Internal(String),
    #[error("Object not found")]
    NotFound,
}

#[async_trait]
pub trait FileStorage {
    async fn put(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), FileStorageError>;

    async fn get(&self, key: &str) -> Result<Vec<u8>, FileStorageError>;

    async fn delete(&self, key: &str) -> Result<(), FileStorageError>;
}
