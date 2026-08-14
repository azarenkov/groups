use std::sync::Arc;

use domain::{
    attachment::repository::AttachmentRepository,
    shared::repository::error::RepositoryError,
    user::{id::UserId, repository::UserRepository},
};

use crate::ports::file_storage::{FileStorage, FileStorageError};

#[derive(Debug, thiserror::Error)]
pub enum UpdateUserAvatarError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
}

pub struct UpdateUserAvatar<U, A, FS>
where
    U: UserRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    user_repository: Arc<U>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
}

impl<U, A, FS> UpdateUserAvatar<U, A, FS>
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

    pub async fn execute(
        &self,
        user_id: UserId,
        bytes: Vec<u8>,
    ) -> Result<(), UpdateUserAvatarError> {
        let key = format!("avatars/{}/{}.png", user_id.uuid(), uuid::Uuid::new_v4());
        self.storage.put(&key, "image/png", bytes).await?;

        let old = match self.user_repository.set_avatar_key(&user_id, Some(&key)).await {
            Ok(old) => old,
            Err(e) => {
                if let Err(qe) = self
                    .attachment_repository
                    .enqueue_cleanup(std::slice::from_ref(&key))
                    .await
                {
                    tracing::warn!(error = %qe, %key, "Failed to enqueue orphaned avatar object");
                }
                cleanup_object(&*self.storage, &*self.attachment_repository, &key).await;
                return Err(e.into());
            }
        };

        if let Some(old_key) = old {
            cleanup_object(&*self.storage, &*self.attachment_repository, &old_key).await;
        }
        Ok(())
    }
}

pub(crate) async fn cleanup_object<FS: FileStorage + ?Sized, A: AttachmentRepository + ?Sized>(
    storage: &FS,
    attachment_repository: &A,
    key: &str,
) {
    match storage.delete(key).await {
        Ok(()) | Err(FileStorageError::NotFound) => {
            if let Err(e) = attachment_repository.dequeue_cleanup(key).await {
                tracing::warn!(error = %e, %key, "Failed to dequeue cleaned object");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, %key, "Failed to delete object; cleanup queue will retry");
        }
    }
}
