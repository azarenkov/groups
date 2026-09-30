use std::sync::Arc;

use media_domain::{attachment::repository::AttachmentRepository, media::repository::MediaRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::ports::{
    file_storage::{FileStorage, FileStorageError},
    image_moderation::{ImageModeration, ImageModerationError},
};

#[derive(Debug, thiserror::Error)]
pub enum UpdateUserAvatarError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Moderation error: {0}")]
    Moderation(#[from] ImageModerationError),
}

pub struct NormalizedImage {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
    pub content_type: &'static str,
    pub moderation_samples: Vec<Vec<u8>>,
    pub still: Option<Vec<u8>>,
}

pub fn still_avatar_key(key: &str) -> Option<String> {
    key.strip_suffix(".gif").map(|stem| format!("{stem}.still.png"))
}

impl NormalizedImage {
    pub fn png(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            extension: "png",
            content_type: "image/png",
            moderation_samples: Vec::new(),
            still: None,
        }
    }
}

pub struct UpdateUserAvatar<M, A, FS, IM>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
{
    media_repository: Arc<M>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
    moderation: Arc<IM>,
}

impl<M, A, FS, IM> UpdateUserAvatar<M, A, FS, IM>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
{
    pub fn new(
        media_repository: Arc<M>,
        attachment_repository: Arc<A>,
        storage: Arc<FS>,
        moderation: Arc<IM>,
    ) -> Self {
        Self {
            media_repository,
            attachment_repository,
            storage,
            moderation,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        image: NormalizedImage,
    ) -> Result<(), UpdateUserAvatarError> {
        if image.moderation_samples.is_empty() {
            self.moderation.ensure_allowed(&image.bytes).await?;
        }
        for sample in &image.moderation_samples {
            self.moderation.ensure_allowed(sample).await?;
        }
        let key = format!(
            "avatars/{}/{}.{}",
            user_id.uuid(),
            uuid::Uuid::new_v4(),
            image.extension
        );
        self.storage.put(&key, image.content_type, image.bytes).await?;
        if let (Some(still), Some(still_key)) = (image.still, still_avatar_key(&key))
            && let Err(e) = self.storage.put(&still_key, "image/png", still).await
        {
            if let Err(qe) = self
                .attachment_repository
                .enqueue_cleanup(std::slice::from_ref(&key))
                .await
            {
                tracing::warn!(error = %qe, %key, "Failed to enqueue the avatar whose still frame failed");
            }
            cleanup_object(&*self.storage, &*self.attachment_repository, &key).await;
            return Err(e.into());
        }

        let old = match self.media_repository.set_user_avatar_key(&user_id, Some(&key)).await {
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
    if let Some(still_key) = still_avatar_key(key) {
        match storage.delete(&still_key).await {
            Ok(()) | Err(FileStorageError::NotFound) => {}
            Err(e) => {
                tracing::warn!(error = %e, key = %still_key, "Failed to delete the still avatar frame; the queue will retry the pair");
                if let Err(qe) = attachment_repository
                    .enqueue_cleanup(&[key.to_string()])
                    .await
                {
                    tracing::warn!(error = %qe, %key, "Failed to enqueue the animated avatar for cleanup");
                }
                return;
            }
        }
    }
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

#[cfg(test)]
mod still_key_tests {
    use super::still_avatar_key;

    #[test]
    fn only_animated_avatars_have_a_still_frame() {
        assert_eq!(
            still_avatar_key("avatars/u/1.gif").as_deref(),
            Some("avatars/u/1.still.png")
        );
        assert_eq!(still_avatar_key("avatars/u/1.png"), None);
    }
}
