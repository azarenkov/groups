use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use media_domain::{attachment::repository::AttachmentRepository, banner::BannerRepository};
use ports::repository::RepositoryError;

use crate::ports::{
    file_storage::{ByteStream, FileStorage, FileStorageError},
    image_moderation::{ImageModeration, ImageModerationError},
    plan_directory::PlanDirectory,
};
use crate::usecases::user::update_avatar::cleanup_object;

#[derive(Debug, thiserror::Error)]
pub enum BannerError {
    #[error("A paid plan is required")]
    ProRequired,
    #[error("Banner not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Moderation error: {0}")]
    Moderation(#[from] ImageModerationError),
}

pub struct UpdateUserBanner<B, A, FS, IM>
where
    B: BannerRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
{
    banners: Arc<B>,
    plans: Arc<dyn PlanDirectory>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
    moderation: Arc<IM>,
}

impl<B, A, FS, IM> UpdateUserBanner<B, A, FS, IM>
where
    B: BannerRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
{
    pub fn new(
        banners: Arc<B>,
        plans: Arc<dyn PlanDirectory>,
        attachment_repository: Arc<A>,
        storage: Arc<FS>,
        moderation: Arc<IM>,
    ) -> Self {
        Self {
            banners,
            plans,
            attachment_repository,
            storage,
            moderation,
        }
    }

    pub async fn ensure_allowed(&self, user_id: &UserId) -> Result<(), BannerError> {
        if self.plans.tier_of(user_id, Utc::now()).await?.is_paid() {
            Ok(())
        } else {
            Err(BannerError::ProRequired)
        }
    }

    pub async fn execute(&self, user_id: UserId, bytes: Vec<u8>) -> Result<(), BannerError> {
        self.ensure_allowed(&user_id).await?;
        self.moderation.ensure_allowed(&bytes).await?;
        let key = format!("banners/{}/{}.png", user_id.uuid(), uuid::Uuid::new_v4());
        self.storage.put(&key, "image/png", bytes).await?;

        let old = match self.banners.set_banner_key(&user_id, Some(&key)).await {
            Ok(old) => old,
            Err(e) => {
                if let Err(qe) = self
                    .attachment_repository
                    .enqueue_cleanup(std::slice::from_ref(&key))
                    .await
                {
                    tracing::warn!(error = %qe, %key, "Failed to enqueue orphaned banner object");
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

    pub async fn remove(&self, user_id: UserId) -> Result<(), BannerError> {
        if let Some(old_key) = self.banners.set_banner_key(&user_id, None).await? {
            cleanup_object(&*self.storage, &*self.attachment_repository, &old_key).await;
        }
        Ok(())
    }
}

pub struct GetUserBanner<B, FS>
where
    B: BannerRepository,
    FS: FileStorage,
{
    banners: Arc<B>,
    plans: Arc<dyn PlanDirectory>,
    storage: Arc<FS>,
}

impl<B, FS> GetUserBanner<B, FS>
where
    B: BannerRepository,
    FS: FileStorage,
{
    pub fn new(banners: Arc<B>, plans: Arc<dyn PlanDirectory>, storage: Arc<FS>) -> Self {
        Self { banners, plans, storage }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<ByteStream, BannerError> {
        if !self.plans.tier_of(&user_id, Utc::now()).await?.is_paid() {
            return Err(BannerError::NotFound);
        }
        let key = self
            .banners
            .get_banner_key(&user_id)
            .await?
            .ok_or(BannerError::NotFound)?;
        self.storage.get_stream(&key).await.map_err(|e| match e {
            FileStorageError::NotFound => BannerError::NotFound,
            other => BannerError::Storage(other),
        })
    }
}
