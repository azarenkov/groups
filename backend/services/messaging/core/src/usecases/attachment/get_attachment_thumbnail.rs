use std::sync::Arc;

use domain::user::id::UserId;
use futures::{StreamExt, stream};
use messaging_domain::message::repository::MessageRepository;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::{
    file_storage::{ByteStream, FileStorage, FileStorageError},
    thumbnailer::{Thumbnail, Thumbnailer},
};

pub const THUMBNAIL_SOURCE_MAX_BYTES: i64 = 25 * 1024 * 1024;

const VARIANTS: [(&str, &str); 2] = [("jpg", "image/jpeg"), ("png", "image/png")];

pub fn thumbnail_key(storage_key: &str, extension: &str) -> String {
    format!("{storage_key}.thumb.{extension}")
}

pub fn thumbnail_keys(storage_key: &str) -> Vec<String> {
    VARIANTS.iter().map(|(ext, _)| thumbnail_key(storage_key, ext)).collect()
}

#[derive(Debug, thiserror::Error)]
pub enum GetAttachmentThumbnailError {
    #[error("Attachment not found")]
    NotFound,
    #[error("No thumbnail for this attachment")]
    Unavailable,
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GetAttachmentThumbnail<M: MessageRepository, S: FileStorage, T: Thumbnailer> {
    message_repository: Arc<M>,
    storage: Arc<S>,
    thumbnailer: Arc<T>,
}

impl<M: MessageRepository, S: FileStorage, T: Thumbnailer> GetAttachmentThumbnail<M, S, T> {
    pub fn new(message_repository: Arc<M>, storage: Arc<S>, thumbnailer: Arc<T>) -> Self {
        Self {
            message_repository,
            storage,
            thumbnailer,
        }
    }

    pub async fn execute(
        &self,
        attachment_id: Uuid,
        user_id: UserId,
    ) -> Result<(&'static str, ByteStream), GetAttachmentThumbnailError> {
        let link = self
            .message_repository
            .find_attachment_for_user(attachment_id, &user_id)
            .await?
            .ok_or(GetAttachmentThumbnailError::NotFound)?;
        if !link.content_type.starts_with("image/") || link.size_bytes > THUMBNAIL_SOURCE_MAX_BYTES {
            return Err(GetAttachmentThumbnailError::Unavailable);
        }
        for (ext, content_type) in VARIANTS {
            match self.storage.get_stream(&thumbnail_key(&link.storage_key, ext)).await {
                Ok(stream) => return Ok((content_type, stream)),
                Err(FileStorageError::NotFound) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let original = self.read_all(&link.storage_key).await?;
        let thumbnail = self
            .thumbnailer
            .thumbnail(original)
            .await
            .ok_or(GetAttachmentThumbnailError::Unavailable)?;
        let ext = VARIANTS
            .iter()
            .find(|(_, ct)| *ct == thumbnail.content_type)
            .map(|(ext, _)| *ext)
            .unwrap_or("jpg");
        self.persist(attachment_id, &user_id, &link.storage_key, ext, &thumbnail).await;
        let body: ByteStream = Box::pin(stream::once(async move { Ok(thumbnail.bytes) }));
        Ok((thumbnail.content_type, body))
    }

    async fn still_linked(&self, attachment_id: Uuid, user_id: &UserId) -> bool {
        matches!(
            self.message_repository.find_attachment_for_user(attachment_id, user_id).await,
            Ok(Some(_))
        )
    }

    async fn persist(&self, attachment_id: Uuid, user_id: &UserId, storage_key: &str, ext: &str, thumbnail: &Thumbnail) {
        if !self.still_linked(attachment_id, user_id).await {
            return;
        }
        let key = thumbnail_key(storage_key, ext);
        if let Err(e) = self.storage.put(&key, thumbnail.content_type, thumbnail.bytes.clone()).await {
            tracing::warn!(error = %e, %attachment_id, "Failed to store attachment thumbnail");
            return;
        }
        if !self.still_linked(attachment_id, user_id).await
            && let Err(e) = self.storage.delete(&key).await
        {
            tracing::warn!(error = %e, %attachment_id, "Failed to drop a thumbnail of a removed attachment");
        }
    }

    async fn read_all(&self, key: &str) -> Result<Vec<u8>, GetAttachmentThumbnailError> {
        let mut stream = self.storage.get_stream(key).await?;
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if bytes.len() + chunk.len() > THUMBNAIL_SOURCE_MAX_BYTES as usize {
                return Err(GetAttachmentThumbnailError::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}
