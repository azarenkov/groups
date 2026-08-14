use std::sync::Arc;

use domain::{
    shared::repository::error::RepositoryError,
    user::{id::UserId, repository::UserRepository},
};

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

pub struct GetUserAvatar<U, FS>
where
    U: UserRepository,
    FS: FileStorage,
{
    user_repository: Arc<U>,
    storage: Arc<FS>,
}

impl<U, FS> GetUserAvatar<U, FS>
where
    U: UserRepository,
    FS: FileStorage,
{
    pub fn new(user_repository: Arc<U>, storage: Arc<FS>) -> Self {
        Self {
            user_repository,
            storage,
        }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<ByteStream, GetUserAvatarError> {
        let key = self
            .user_repository
            .get_avatar_key(&user_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => GetUserAvatarError::NotFound,
                other => GetUserAvatarError::Repository(other),
            })?
            .ok_or(GetUserAvatarError::NotFound)?;
        self.storage.get_stream(&key).await.map_err(|e| match e {
            FileStorageError::NotFound => GetUserAvatarError::NotFound,
            other => GetUserAvatarError::Storage(other),
        })
    }
}
