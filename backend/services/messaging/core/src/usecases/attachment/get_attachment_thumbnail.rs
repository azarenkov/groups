use std::sync::Arc;

use domain::{
    attachment::{THUMBNAIL_VARIANTS, thumbnail_key},
    user::id::UserId,
};
use futures::stream;
use messaging_domain::message::repository::MessageRepository;
use ports::repository::RepositoryError;
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::derived_objects::{DerivedObjects, KeyLock, SourceRead};
use crate::ports::{
    file_storage::{ByteStream, FileStorage, FileStorageError},
    storage_releases::StorageReleases,
    thumbnailer::Thumbnailer,
};

pub const THUMBNAIL_SOURCE_MAX_BYTES: i64 = 25 * 1024 * 1024;
pub const THUMBNAIL_CONCURRENCY: usize = 2;

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

struct Inner<M, S, T, R> {
    objects: DerivedObjects<M, S, R>,
    thumbnailer: Arc<T>,
    generating: Arc<Semaphore>,
}

pub struct GetAttachmentThumbnail<M, S, T, R> {
    inner: Arc<Inner<M, S, T, R>>,
}

impl<M, S, T, R> GetAttachmentThumbnail<M, S, T, R>
where
    M: MessageRepository + Send + Sync + 'static,
    S: FileStorage + Send + Sync + 'static,
    T: Thumbnailer + Send + Sync + 'static,
    R: StorageReleases + Send + Sync + 'static,
{
    pub fn new(message_repository: Arc<M>, storage: Arc<S>, thumbnailer: Arc<T>, releases: Arc<R>) -> Self {
        Self {
            inner: Arc::new(Inner {
                objects: DerivedObjects::new(message_repository, storage, releases),
                thumbnailer,
                generating: Arc::new(Semaphore::new(THUMBNAIL_CONCURRENCY)),
            }),
        }
    }

    pub async fn execute(
        &self,
        attachment_id: Uuid,
        user_id: UserId,
    ) -> Result<(&'static str, ByteStream), GetAttachmentThumbnailError> {
        let inner = &self.inner;
        let link = inner
            .objects
            .linked(attachment_id, &user_id)
            .await?
            .ok_or(GetAttachmentThumbnailError::NotFound)?;
        if !link.content_type.starts_with("image/") || link.size_bytes > THUMBNAIL_SOURCE_MAX_BYTES {
            return Err(GetAttachmentThumbnailError::Unavailable);
        }
        if let Some(cached) = inner.cached(&link.storage_key).await? {
            return Ok(cached);
        }
        let key_lock = inner.objects.lock(&link.storage_key);
        let worker = inner.clone();
        let storage_key = link.storage_key.clone();
        tokio::spawn(async move { worker.generate(attachment_id, user_id, storage_key, key_lock).await })
            .await
            .map_err(|_| GetAttachmentThumbnailError::Unavailable)?
    }
}

impl<M, S, T, R> Inner<M, S, T, R>
where
    M: MessageRepository + Send + Sync,
    S: FileStorage + Send + Sync,
    T: Thumbnailer + Send + Sync,
    R: StorageReleases + Send + Sync,
{
    async fn cached(&self, storage_key: &str) -> Result<Option<(&'static str, ByteStream)>, GetAttachmentThumbnailError> {
        for (ext, content_type) in THUMBNAIL_VARIANTS {
            if let Some(stream) = self.objects.open(&thumbnail_key(storage_key, ext)).await? {
                return Ok(Some((content_type, stream)));
            }
        }
        Ok(None)
    }

    async fn generate(
        &self,
        attachment_id: Uuid,
        user_id: UserId,
        storage_key: String,
        key_lock: KeyLock,
    ) -> Result<(&'static str, ByteStream), GetAttachmentThumbnailError> {
        let _key_guard = key_lock.hold().await;
        if let Some(cached) = self.cached(&storage_key).await? {
            return Ok(cached);
        }
        let _permit = self
            .generating
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| GetAttachmentThumbnailError::Unavailable)?;
        let SourceRead::Bytes(original) = self.objects.read_source(&storage_key, THUMBNAIL_SOURCE_MAX_BYTES).await? else {
            return Err(GetAttachmentThumbnailError::Unavailable);
        };
        let thumbnail = self
            .thumbnailer
            .thumbnail(original)
            .await
            .ok_or(GetAttachmentThumbnailError::Unavailable)?;
        let ext = THUMBNAIL_VARIANTS
            .iter()
            .find(|(_, ct)| *ct == thumbnail.content_type)
            .map(|(ext, _)| *ext)
            .unwrap_or("jpg");
        self.objects
            .persist(
                attachment_id,
                &user_id,
                &storage_key,
                &thumbnail_key(&storage_key, ext),
                thumbnail.content_type,
                thumbnail.bytes.clone(),
            )
            .await;
        let body: ByteStream = Box::pin(stream::once(async move { Ok(thumbnail.bytes) }));
        Ok((thumbnail.content_type, body))
    }
}
