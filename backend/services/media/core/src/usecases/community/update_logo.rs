use std::sync::Arc;

use media_domain::{attachment::repository::AttachmentRepository, media::repository::MediaRepository};
use domain::{community::id::CommunityId, user::id::UserId};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        file_storage::{FileStorage, FileStorageError},
        image_moderation::{ImageModeration, ImageModerationError},
        community_owner_lookup::{CommunityOwnerLookup, CommunityOwnerLookupError},
    },
    usecases::user::update_avatar::cleanup_object,
};

#[derive(Debug, thiserror::Error)]
pub enum UpdateCommunityLogoError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Moderation error: {0}")]
    Moderation(#[from] ImageModerationError),
    #[error("Community not found")]
    CommunityNotFound,
    #[error("Not the community owner")]
    NotOwner,
    #[error("Owner lookup unavailable: {0}")]
    OwnerLookup(#[from] CommunityOwnerLookupError),
}

pub struct UpdateCommunityLogo<M, A, FS, IM, O>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
    O: CommunityOwnerLookup,
{
    media_repository: Arc<M>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
    moderation: Arc<IM>,
    owner_lookup: Arc<O>,
}

impl<M, A, FS, IM, O> UpdateCommunityLogo<M, A, FS, IM, O>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
    O: CommunityOwnerLookup,
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
        community_id: CommunityId,
        requester_id: UserId,
        bytes: Option<Vec<u8>>,
    ) -> Result<(), UpdateCommunityLogoError> {
        let owner = match self.media_repository.get_community_owner(&community_id).await? {
            Some(owner) => owner,
            None => {
                let owner = self
                    .owner_lookup
                    .find_owner(&community_id)
                    .await?
                    .ok_or(UpdateCommunityLogoError::CommunityNotFound)?;
                if let Err(e) = self
                    .media_repository
                    .upsert_community_owner(&community_id, &owner, chrono::Utc::now())
                    .await
                {
                    tracing::warn!(error = %e, "Failed to backfill community owner projection");
                }
                owner
            }
        };
        if owner != requester_id {
            let allowed = self
                .owner_lookup
                .can_manage(&community_id, &requester_id)
                .await?;
            if !allowed {
                return Err(UpdateCommunityLogoError::NotOwner);
            }
        }

        let new_key = match bytes {
            Some(bytes) => {
                self.moderation.ensure_allowed(&bytes).await?;
                let key = format!("logos/{}/{}.png", community_id.uuid(), uuid::Uuid::new_v4());
                self.storage.put(&key, "image/png", bytes).await?;
                Some(key)
            }
            None => None,
        };

        if owner != requester_id {
            let still_allowed = self
                .owner_lookup
                .can_manage(&community_id, &requester_id)
                .await
                .unwrap_or_default();
            if !still_allowed {
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
                return Err(UpdateCommunityLogoError::NotOwner);
            }
        }

        let old = match self
            .media_repository
            .set_community_logo_key(&community_id, new_key.as_deref())
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
                    RepositoryError::NotFound => UpdateCommunityLogoError::CommunityNotFound,
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
