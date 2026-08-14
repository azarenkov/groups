use std::sync::Arc;

use domain::{
    attachment::repository::AttachmentRepository,
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::{
    ports::file_storage::{FileStorage, FileStorageError},
    usecases::user::update_avatar::cleanup_object,
};

#[derive(Debug, thiserror::Error)]
pub enum UpdateServerLogoError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Server not found")]
    ServerNotFound,
    #[error("Not the server owner")]
    NotOwner,
}

pub struct UpdateServerLogo<S, A, FS>
where
    S: ServerRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    server_repository: Arc<S>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
}

impl<S, A, FS> UpdateServerLogo<S, A, FS>
where
    S: ServerRepository,
    A: AttachmentRepository,
    FS: FileStorage,
{
    pub fn new(
        server_repository: Arc<S>,
        attachment_repository: Arc<A>,
        storage: Arc<FS>,
    ) -> Self {
        Self {
            server_repository,
            attachment_repository,
            storage,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        requester_id: UserId,
        bytes: Option<Vec<u8>>,
    ) -> Result<(), UpdateServerLogoError> {
        let owner = self
            .server_repository
            .find_server_owner(&server_id)
            .await?
            .ok_or(UpdateServerLogoError::ServerNotFound)?;
        if owner != requester_id {
            return Err(UpdateServerLogoError::NotOwner);
        }

        let new_key = match bytes {
            Some(bytes) => {
                let key = format!("logos/{}/{}.png", server_id.uuid(), uuid::Uuid::new_v4());
                self.storage.put(&key, "image/png", bytes).await?;
                Some(key)
            }
            None => None,
        };

        let old = match self
            .server_repository
            .set_logo_key(&server_id, new_key.as_deref())
            .await
        {
            Ok(old) => old,
            Err(e) => {
                if let Some(key) = new_key {
                    if let Err(qe) = self
                        .attachment_repository
                        .enqueue_cleanup(std::slice::from_ref(&key))
                        .await
                    {
                        tracing::warn!(error = %qe, %key, "Failed to enqueue orphaned logo object");
                    }
                    cleanup_object(&*self.storage, &*self.attachment_repository, &key).await;
                }
                return Err(e.into());
            }
        };

        if let Some(old_key) = old {
            cleanup_object(&*self.storage, &*self.attachment_repository, &old_key).await;
        }
        Ok(())
    }
}
