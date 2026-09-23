use std::sync::Arc;

use media_domain::media::repository::MediaRepository;
use domain::community::id::CommunityId;
use ports::repository::RepositoryError;

use crate::ports::file_storage::{ByteStream, FileStorage, FileStorageError};

#[derive(Debug, thiserror::Error)]
pub enum GetCommunityLogoError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(FileStorageError),
    #[error("Logo not found")]
    NotFound,
}

pub struct GetCommunityLogo<S, FS>
where
    S: MediaRepository,
    FS: FileStorage,
{
    media_repository: Arc<S>,
    storage: Arc<FS>,
}

impl<S, FS> GetCommunityLogo<S, FS>
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

    pub async fn execute(&self, community_id: CommunityId) -> Result<ByteStream, GetCommunityLogoError> {
        let key = self
            .media_repository
            .get_community_logo_key(&community_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => GetCommunityLogoError::NotFound,
                other => GetCommunityLogoError::Repository(other),
            })?
            .ok_or(GetCommunityLogoError::NotFound)?;
        self.storage.get_stream(&key).await.map_err(|e| match e {
            FileStorageError::NotFound => GetCommunityLogoError::NotFound,
            other => GetCommunityLogoError::Storage(other),
        })
    }
}
