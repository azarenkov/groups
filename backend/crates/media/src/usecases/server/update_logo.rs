use std::sync::Arc;

use domain::{
    attachment::repository::AttachmentRepository, media::repository::MediaRepository,
    server::id::ServerId, shared::repository::error::RepositoryError, user::id::UserId,
};

use crate::{
    ports::{
        file_storage::{FileStorage, FileStorageError},
        image_moderation::{ImageModeration, ImageModerationError},
        server_owner_lookup::{ServerOwnerLookup, ServerOwnerLookupError},
    },
    usecases::user::update_avatar::cleanup_object,
};

#[derive(Debug, thiserror::Error)]
pub enum UpdateServerLogoError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Moderation error: {0}")]
    Moderation(#[from] ImageModerationError),
    #[error("Server not found")]
    ServerNotFound,
    #[error("Not the server owner")]
    NotOwner,
    #[error("Owner lookup unavailable: {0}")]
    OwnerLookup(#[from] ServerOwnerLookupError),
}

pub struct UpdateServerLogo<M, A, FS, IM, O>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
    O: ServerOwnerLookup,
{
    media_repository: Arc<M>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
    moderation: Arc<IM>,
    owner_lookup: Arc<O>,
}

impl<M, A, FS, IM, O> UpdateServerLogo<M, A, FS, IM, O>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
    O: ServerOwnerLookup,
{
    pub fn new(
        media_repository: Arc<M>,
        attachment_repository: Arc<A>,
        storage: Arc<FS>,
        moderation: Arc<IM>,
        owner_lookup: Arc<O>,
    ) -> Self {
        Self {
            media_repository,
            attachment_repository,
            storage,
            moderation,
            owner_lookup,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        requester_id: UserId,
        bytes: Option<Vec<u8>>,
    ) -> Result<(), UpdateServerLogoError> {
        let owner = match self.media_repository.get_server_owner(&server_id).await? {
            Some(owner) => owner,
            None => {
                let owner = self
                    .owner_lookup
                    .find_owner(&server_id)
                    .await?
                    .ok_or(UpdateServerLogoError::ServerNotFound)?;
                if let Err(e) = self
                    .media_repository
                    .upsert_server_owner(&server_id, &owner, chrono::Utc::now())
                    .await
                {
                    tracing::warn!(error = %e, "Failed to backfill server owner projection");
                }
                owner
            }
        };
        if owner != requester_id {
            return Err(UpdateServerLogoError::NotOwner);
        }

        let new_key = match bytes {
            Some(bytes) => {
                self.moderation.ensure_allowed(&bytes).await?;
                let key = format!("logos/{}/{}.png", server_id.uuid(), uuid::Uuid::new_v4());
                self.storage.put(&key, "image/png", bytes).await?;
                Some(key)
            }
            None => None,
        };

        let old = match self
            .media_repository
            .set_server_logo_key(&server_id, new_key.as_deref())
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
                return Err(match e {
                    RepositoryError::NotFound => UpdateServerLogoError::ServerNotFound,
                    e => e.into(),
                });
            }
        };

        if let Some(old_key) = old {
            cleanup_object(&*self.storage, &*self.attachment_repository, &old_key).await;
        }
        Ok(())
    }
}
