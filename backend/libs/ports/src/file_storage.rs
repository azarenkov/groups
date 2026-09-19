use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

#[derive(Debug, thiserror::Error)]
pub enum FileStorageError {
    #[error("Storage error: {0}")]
    Internal(String),
    #[error("Object not found")]
    NotFound,
}

pub type ByteStream =
    Pin<Box<dyn Stream<Item = Result<Vec<u8>, FileStorageError>> + Send>>;

#[async_trait]
pub trait FileStorage {
    async fn put(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), FileStorageError>;

    async fn put_stream(
        &self,
        key: &str,
        content_type: &str,
        stream: ByteStream,
    ) -> Result<u64, FileStorageError>;

    async fn get_stream(&self, key: &str) -> Result<ByteStream, FileStorageError>;

    async fn delete(&self, key: &str) -> Result<(), FileStorageError>;
}
