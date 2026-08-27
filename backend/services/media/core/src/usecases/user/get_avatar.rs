use std::sync::Arc;

use media_domain::media::repository::MediaRepository;
use domain::{shared::repository::error::RepositoryError, user::id::UserId};

use crate::ports::file_storage::{ByteStream, FileStorage, FileStorageError};

#[derive(Debug, thiserror::Error)]
pub enum GetUserAvatarError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(FileStorageError),
    #[error("Avatar not found")]
    NotFound,
}

pub struct GetUserAvatar<M, FS>
where
    M: MediaRepository,
    FS: FileStorage,
{
    media_repository: Arc<M>,
    storage: Arc<FS>,
}

impl<M, FS> GetUserAvatar<M, FS>
where
    M: MediaRepository,
    FS: FileStorage,
{
    pub fn new(media_repository: Arc<M>, storage: Arc<FS>) -> Self {
        Self {
            media_repository,
            storage,
        }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<ByteStream, GetUserAvatarError> {
        let key = self
            .media_repository
            .get_user_avatar_key(&user_id)
            .await?
            .ok_or(GetUserAvatarError::NotFound)?;
        self.storage.get_stream(&key).await.map_err(|e| match e {
            FileStorageError::NotFound => GetUserAvatarError::NotFound,
            other => GetUserAvatarError::Storage(other),
        })
    }
}
