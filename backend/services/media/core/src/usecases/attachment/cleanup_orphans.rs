use std::sync::Arc;

use media_domain::attachment::repository::AttachmentRepository;

use crate::usecases::user::update_avatar::still_avatar_key;

use crate::ports::{
    attachment_links_lookup::AttachmentLinksLookup,
    file_storage::{FileStorage, FileStorageError},
};

pub fn attachment_thumbnail_keys(key: &str) -> Vec<String> {
    if !key.starts_with("attachments/") {
        return Vec::new();
    }
    ["jpg", "png"].iter().map(|ext| format!("{key}.thumb.{ext}")).collect()
}

pub struct CleanupOrphanAttachments<R: AttachmentRepository, S: FileStorage, L: AttachmentLinksLookup>
{
    repository: Arc<R>,
    storage: Arc<S>,
    links_lookup: Arc<L>,
    stale_claim_cursor: std::sync::Mutex<Option<(chrono::DateTime<chrono::Utc>, uuid::Uuid)>>,
}

impl<R: AttachmentRepository, S: FileStorage, L: AttachmentLinksLookup>
    CleanupOrphanAttachments<R, S, L>
{
    async fn delete_thumbnails(&self, key: &str) -> bool {
        let mut all = true;
        for thumb_key in attachment_thumbnail_keys(key) {
            match self.storage.delete(&thumb_key).await {
                Ok(()) | Err(FileStorageError::NotFound) => {}
                Err(e) => {
                    tracing::warn!(error = %e, key = %thumb_key, "Failed to delete attachment thumbnail");
                    all = false;
                }
            }
        }
        all
    }

    pub fn new(repository: Arc<R>, storage: Arc<S>, links_lookup: Arc<L>) -> Self {
        Self {
            repository,
            storage,
            links_lookup,
            stale_claim_cursor: std::sync::Mutex::new(None),
        }
    }

    fn store_stale_claim_cursor(
        &self,
        cursor: Option<(chrono::DateTime<chrono::Utc>, uuid::Uuid)>,
    ) {
        if let Ok(mut slot) = self.stale_claim_cursor.lock() {
            *slot = cursor;
        }
    }

    async fn reconcile_stale_claims(&self) {
        const PAGE_SIZE: i64 = 500;
        const MAX_PAGES: usize = 20;
        let mut cursor = match self.stale_claim_cursor.lock() {
            Ok(slot) => *slot,
            Err(_) => None,
        };
        for _ in 0..MAX_PAGES {
            let page = match self.repository.list_stale_claims(2, PAGE_SIZE, cursor).await {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::warn!(error = %e, "Stale claim query failed");
                    self.store_stale_claim_cursor(cursor);
                    return;
                }
            };
            let Some(last) = page.last() else {
                self.store_stale_claim_cursor(None);
                return;
            };
            let prev_cursor = cursor;
            cursor = Some((last.1, last.0));
            let page_len = page.len();
            let ids: Vec<uuid::Uuid> = page.iter().map(|(id, _)| *id).collect();
            let linked: std::collections::HashSet<uuid::Uuid> =
                match self.links_lookup.linked_subset(&ids).await {
                    Ok(linked) => linked.into_iter().collect(),
                    Err(e) => {
                        tracing::warn!(error = %e, "Linked attachment lookup failed; will retry next sweep");
                        self.store_stale_claim_cursor(prev_cursor);
                        return;
                    }
                };
            let orphaned: Vec<uuid::Uuid> = ids
                .into_iter()
                .filter(|id| !linked.contains(id))
                .collect();
            if !orphaned.is_empty() {
                match self.repository.release_claims(&orphaned).await {
                    Ok(()) => {
                        tracing::info!(count = orphaned.len(), "Released stale attachment claims");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to release stale attachment claims");
                    }
                }
            }
            if (page_len as i64) < PAGE_SIZE {
                self.store_stale_claim_cursor(None);
                return;
            }
        }
        self.store_stale_claim_cursor(cursor);
    }

    pub async fn run_once(&self) {
        self.reconcile_stale_claims().await;

        match self.repository.list_orphans_older_than(24, 1000).await {
            Ok(orphans) => {
                let orphans = if orphans.is_empty() {
                    orphans
                } else {
                    match self.links_lookup.linked_subset(&orphans).await {
                        Ok(linked) => {
                            let linked: std::collections::HashSet<uuid::Uuid> =
                                linked.into_iter().collect();
                            if !linked.is_empty() {
                                let to_restore: Vec<uuid::Uuid> =
                                    linked.iter().copied().collect();
                                match self.repository.restore_claims(&to_restore).await {
                                    Ok(()) => {
                                        tracing::warn!(count = to_restore.len(), "Restored claims for linked attachments found unclaimed");
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, "Failed to restore claims for linked attachments");
                                    }
                                }
                            }
                            orphans.into_iter().filter(|id| !linked.contains(id)).collect()
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Linked attachment lookup failed; skipping orphan deletion this sweep");
                            Vec::new()
                        }
                    }
                };
                for id in orphans {
                    let key = match self.repository.claim_orphan(id).await {
                        Ok(Some(key)) => key,
                        Ok(None) => continue,
                        Err(e) => {
                            tracing::warn!(error = %e, %id, "Failed to claim orphan attachment");
                            continue;
                        }
                    };
                    if !self.delete_thumbnails(&key).await
                        && let Err(e) = self.repository.enqueue_cleanup(std::slice::from_ref(&key)).await
                    {
                        tracing::warn!(error = %e, %key, "Failed to queue thumbnails of an orphan attachment; keeping it for the next sweep");
                        continue;
                    }
                    match self.storage.delete(&key).await {
                        Ok(()) | Err(FileStorageError::NotFound) => {
                            if let Err(e) = self.repository.delete_orphan_row(id).await {
                                tracing::warn!(error = %e, %key, "Failed to delete orphan attachment row");
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, %key, "Failed to delete orphan attachment object; will retry");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Orphan attachment cleanup query failed");
            }
        }

        match self.repository.list_cleanup_queue(1000).await {
            Ok(keys) => {
                'keys: for key in keys {
                    if let Some(still_key) = still_avatar_key(&key) {
                        match self.storage.delete(&still_key).await {
                            Ok(()) | Err(FileStorageError::NotFound) => {}
                            Err(e) => {
                                tracing::warn!(error = %e, key = %still_key, "Failed to delete queued still avatar frame; will retry");
                                continue;
                            }
                        }
                    }
                    for thumb_key in attachment_thumbnail_keys(&key) {
                        match self.storage.delete(&thumb_key).await {
                            Ok(()) | Err(FileStorageError::NotFound) => {}
                            Err(e) => {
                                tracing::warn!(error = %e, key = %thumb_key, "Failed to delete queued attachment thumbnail; will retry");
                                continue 'keys;
                            }
                        }
                    }
                    match self.storage.delete(&key).await {
                        Ok(()) | Err(FileStorageError::NotFound) => {
                            match self
                                .repository
                                .delete_by_storage_keys(std::slice::from_ref(&key))
                                .await
                            {
                                Ok(()) => {
                                    if let Err(e) = self.repository.dequeue_cleanup(&key).await {
                                        tracing::warn!(error = %e, %key, "Failed to dequeue cleaned attachment");
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, %key, "Failed to delete attachment row for cleaned object; will retry");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, %key, "Failed to delete queued attachment object; will retry");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Cleanup queue query failed");
            }
        }
    }
}

#[cfg(test)]
mod thumbnail_key_tests {
    use super::attachment_thumbnail_keys;

    #[test]
    fn attachment_keys_also_clean_their_thumbnails() {
        assert_eq!(
            attachment_thumbnail_keys("attachments/abc"),
            vec!["attachments/abc.thumb.jpg".to_string(), "attachments/abc.thumb.png".to_string()]
        );
        assert!(attachment_thumbnail_keys("avatars/abc.png").is_empty());
    }
}
