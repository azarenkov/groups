use std::sync::Arc;

use domain::{
    attachment::repository::AttachmentRepository,
    media::repository::MediaRepository,
    shared::repository::error::RepositoryError,
    user::id::UserId,
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

pub struct DeleteUserAvatar<M, A, FS>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    media_repository: Arc<M>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
}

impl<M, A, FS> DeleteUserAvatar<M, A, FS>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    pub fn new(media_repository: Arc<M>, attachment_repository: Arc<A>, storage: Arc<FS>) -> Self {
        Self {
            media_repository,
            attachment_repository,
            storage,
        }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<(), DeleteUserAvatarError> {
        let old = self.media_repository.set_user_avatar_key(&user_id, None).await?;
        if let Some(old_key) = old {
            cleanup_object(&*self.storage, &*self.attachment_repository, &old_key).await;
        }
        Ok(())
    }
}
