use std::sync::Arc;

use media_domain::media::repository::MediaRepository;
use domain::server::id::ServerId;
use ports::repository::RepositoryError;

use crate::ports::file_storage::{ByteStream, FileStorage, FileStorageError};

#[derive(Debug, thiserror::Error)]
pub enum GetServerLogoError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(FileStorageError),
    #[error("Logo not found")]
    NotFound,
}

pub struct GetServerLogo<S, FS>
where
    S: MediaRepository,
    FS: FileStorage,
{
    media_repository: Arc<S>,
    storage: Arc<FS>,
}

impl<S, FS> GetServerLogo<S, FS>
where
    S: MediaRepository,
    FS: FileStorage,
{
    pub fn new(media_repository: Arc<S>, storage: Arc<FS>) -> Self {
        Self {
            media_repository,
            storage,
        }
    }

    pub async fn execute(&self, server_id: ServerId) -> Result<ByteStream, GetServerLogoError> {
        let key = self
            .media_repository
            .get_server_logo_key(&server_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => GetServerLogoError::NotFound,
                other => GetServerLogoError::Repository(other),
            })?
            .ok_or(GetServerLogoError::NotFound)?;
        self.storage.get_stream(&key).await.map_err(|e| match e {
            FileStorageError::NotFound => GetServerLogoError::NotFound,
            other => GetServerLogoError::Storage(other),
        })
    }
}
