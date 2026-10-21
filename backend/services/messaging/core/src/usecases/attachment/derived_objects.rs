use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use domain::user::id::UserId;
use futures::StreamExt;
use messaging_domain::message::repository::MessageRepository;
use ports::repository::RepositoryError;
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

use crate::ports::{
    file_storage::{ByteStream, FileStorage, FileStorageError},
    storage_releases::StorageReleases,
};
use domain::attachment::AttachmentLink;

type InFlight = StdMutex<HashMap<String, Arc<Mutex<()>>>>;

pub(crate) struct KeyLock {
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

    pub(crate) async fn hold(&self) -> OwnedMutexGuard<()> {
        self.lock.clone().expect("key lock is held until drop").lock_owned().await
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

pub(crate) enum SourceRead {
    Bytes(Vec<u8>),
    TooLarge,
}

pub(crate) struct DerivedObjects<M, S, R> {
    message_repository: Arc<M>,
    storage: Arc<S>,
    releases: Arc<R>,
    in_flight: Arc<InFlight>,
}

impl<M, S, R> DerivedObjects<M, S, R>
where
    M: MessageRepository + Send + Sync,
    S: FileStorage + Send + Sync,
    R: StorageReleases + Send + Sync,
{
    pub(crate) fn new(message_repository: Arc<M>, storage: Arc<S>, releases: Arc<R>) -> Self {
        Self {
            message_repository,
            storage,
            releases,
            in_flight: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    pub(crate) fn storage(&self) -> &S {
        &self.storage
    }

    pub(crate) async fn linked(&self, attachment_id: Uuid, user_id: &UserId) -> Result<Option<AttachmentLink>, RepositoryError> {
        self.message_repository.find_attachment_for_user(attachment_id, user_id).await
    }

    pub(crate) fn lock(&self, source_key: &str) -> KeyLock {
        KeyLock::acquire(&self.in_flight, source_key)
    }

    pub(crate) async fn open(&self, key: &str) -> Result<Option<ByteStream>, FileStorageError> {
        match self.storage.get_stream(key).await {
            Ok(stream) => Ok(Some(stream)),
            Err(FileStorageError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub(crate) async fn read_source(&self, key: &str, max_bytes: i64) -> Result<SourceRead, FileStorageError> {
        let mut stream = self.storage.get_stream(key).await?;
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if bytes.len() + chunk.len() > max_bytes as usize {
                return Ok(SourceRead::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(SourceRead::Bytes(bytes))
    }

    async fn still_linked(&self, attachment_id: Uuid, user_id: &UserId) -> bool {
        matches!(self.linked(attachment_id, user_id).await, Ok(Some(_)))
    }

    pub(crate) async fn persist(
        &self,
        attachment_id: Uuid,
        user_id: &UserId,
        source_key: &str,
        derived_key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) {
        if !self.still_linked(attachment_id, user_id).await {
            return;
        }
        if let Err(e) = self.storage.put(derived_key, content_type, bytes).await {
            tracing::warn!(error = %e, %attachment_id, key = %derived_key, "Failed to store a derived attachment object");
            return;
        }
        if !self.still_linked(attachment_id, user_id).await
            && let Err(e) = self.storage.delete(derived_key).await
        {
            tracing::warn!(error = %e, %attachment_id, key = %derived_key, "Failed to drop a derived object of a removed attachment; queueing cleanup");
            if let Err(e) = self.releases.release(&[source_key.to_string()]).await {
                tracing::error!(error = %e, %attachment_id, "Failed to queue cleanup of a late derived object");
            }
        }
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
