use std::sync::Arc;

use domain::{
    attachment::repository::AttachmentRepository,
    shared::repository::error::RepositoryError,
    user::{id::UserId, repository::UserRepository},
};

use crate::{
    ports::file_storage::{FileStorage, FileStorageError},
    usecases::user::update_avatar::cleanup_object,
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteUserAvatarError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
}

pub struct DeleteUserAvatar<U, A, FS>
where
    U: UserRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    user_repository: Arc<U>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
}

impl<U, A, FS> DeleteUserAvatar<U, A, FS>
where
    U: UserRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    pub fn new(user_repository: Arc<U>, attachment_repository: Arc<A>, storage: Arc<FS>) -> Self {
        Self {
            user_repository,
            attachment_repository,
            storage,
        }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<(), DeleteUserAvatarError> {
        let old = self.user_repository.set_avatar_key(&user_id, None).await?;
        if let Some(old_key) = old {
            cleanup_object(&*self.storage, &*self.attachment_repository, &old_key).await;
        }
        Ok(())
    }
}
