use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use domain::user::id::UserId;
use futures::{StreamExt, stream};
use messaging_domain::message::repository::MessageRepository;
use ports::repository::RepositoryError;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

use crate::ports::{
    file_storage::{ByteStream, FileStorage, FileStorageError},
    storage_releases::StorageReleases,
    thumbnailer::{Thumbnail, Thumbnailer},
};

pub const THUMBNAIL_SOURCE_MAX_BYTES: i64 = 25 * 1024 * 1024;
pub const THUMBNAIL_CONCURRENCY: usize = 2;

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

type InFlight = StdMutex<HashMap<String, Arc<Mutex<()>>>>;

struct KeyLock {
    map: Arc<InFlight>,
    key: String,
    lock: Option<Arc<Mutex<()>>>,
}

impl KeyLock {
    fn acquire(map: &Arc<InFlight>, key: &str) -> Self {
        let lock = map
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(key.to_string())
            .or_default()
            .clone();
        Self {
            map: map.clone(),
            key: key.to_string(),
            lock: Some(lock),
        }
    }

    fn lock(&self) -> Arc<Mutex<()>> {
        self.lock.clone().expect("key lock is held until drop")
    }
}

impl Drop for KeyLock {
    fn drop(&mut self) {
        let mut map = self.map.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        drop(self.lock.take());
        if map.get(&self.key).is_some_and(|entry| Arc::strong_count(entry) == 1) {
            map.remove(&self.key);
        }
    }
}

struct Inner<M, S, T, R> {
    message_repository: Arc<M>,
    storage: Arc<S>,
    thumbnailer: Arc<T>,
    releases: Arc<R>,
    generating: Arc<Semaphore>,
    in_flight: Arc<InFlight>,
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
                message_repository,
                storage,
                thumbnailer,
                releases,
                generating: Arc::new(Semaphore::new(THUMBNAIL_CONCURRENCY)),
                in_flight: Arc::new(StdMutex::new(HashMap::new())),
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
            .message_repository
            .find_attachment_for_user(attachment_id, &user_id)
            .await?
            .ok_or(GetAttachmentThumbnailError::NotFound)?;
        if !link.content_type.starts_with("image/") || link.size_bytes > THUMBNAIL_SOURCE_MAX_BYTES {
            return Err(GetAttachmentThumbnailError::Unavailable);
        }
        if let Some(cached) = inner.cached(&link.storage_key).await? {
            return Ok(cached);
        }
        let key_lock = KeyLock::acquire(&inner.in_flight, &link.storage_key);
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
        for (ext, content_type) in VARIANTS {
            match self.storage.get_stream(&thumbnail_key(storage_key, ext)).await {
                Ok(stream) => return Ok(Some((content_type, stream))),
                Err(FileStorageError::NotFound) => {}
                Err(e) => return Err(e.into()),
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
        let _key_guard = key_lock.lock().lock_owned().await;
        if let Some(cached) = self.cached(&storage_key).await? {
            return Ok(cached);
        }
        let _permit = self
            .generating
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| GetAttachmentThumbnailError::Unavailable)?;
        let original = self.read_all(&storage_key).await?;
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
        self.persist(attachment_id, &user_id, &storage_key, ext, &thumbnail).await;
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
            tracing::warn!(error = %e, %attachment_id, "Failed to drop a thumbnail of a removed attachment; queueing cleanup");
            if let Err(e) = self.releases.release(&[storage_key.to_string()]).await {
                tracing::error!(error = %e, %attachment_id, "Failed to queue cleanup of a late thumbnail");
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_locks_are_removed_when_dropped_even_if_shared() {
        let map: Arc<InFlight> = Arc::new(StdMutex::new(HashMap::new()));
        let first = KeyLock::acquire(&map, "a");
        let second = KeyLock::acquire(&map, "a");
        assert_eq!(map.lock().unwrap().len(), 1);
        drop(first);
        assert_eq!(map.lock().unwrap().len(), 1);
        drop(second);
        assert!(map.lock().unwrap().is_empty());
    }
}
