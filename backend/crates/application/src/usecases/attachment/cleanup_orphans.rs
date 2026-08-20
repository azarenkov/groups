use std::sync::Arc;

use domain::attachment::repository::AttachmentRepository;

use crate::ports::{
    attachment_links_lookup::AttachmentLinksLookup,
    file_storage::{FileStorage, FileStorageError},
};

pub struct CleanupOrphanAttachments<R: AttachmentRepository, S: FileStorage, L: AttachmentLinksLookup>
{
    repository: Arc<R>,
    storage: Arc<S>,
    links_lookup: Arc<L>,
}

impl<R: AttachmentRepository, S: FileStorage, L: AttachmentLinksLookup>
    CleanupOrphanAttachments<R, S, L>
{
    pub fn new(repository: Arc<R>, storage: Arc<S>, links_lookup: Arc<L>) -> Self {
        Self {
            repository,
            storage,
            links_lookup,
        }
    }

    async fn reconcile_stale_claims(&self) {
        let stale = match self.repository.list_stale_claims(2, 500).await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!(error = %e, "Stale claim query failed");
                return;
            }
        };
        if stale.is_empty() {
            return;
        }
        let linked: std::collections::HashSet<uuid::Uuid> =
            match self.links_lookup.linked_subset(&stale).await {
                Ok(ids) => ids.into_iter().collect(),
                Err(e) => {
                    tracing::warn!(error = %e, "Linked attachment lookup failed; will retry next sweep");
                    return;
                }
            };
        let orphaned: Vec<uuid::Uuid> = stale
            .into_iter()
            .filter(|id| !linked.contains(id))
            .collect();
        if orphaned.is_empty() {
            return;
        }
        match self.repository.release_claims(&orphaned).await {
            Ok(()) => {
                tracing::info!(count = orphaned.len(), "Released stale attachment claims");
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to release stale attachment claims");
            }
        }
    }

    pub async fn run_once(&self) {
        self.reconcile_stale_claims().await;

        match self.repository.list_orphans_older_than(24, 1000).await {
            Ok(orphans) => {
                for id in orphans {
                    let key = match self.repository.claim_orphan(id).await {
                        Ok(Some(key)) => key,
                        Ok(None) => continue,
                        Err(e) => {
                            tracing::warn!(error = %e, %id, "Failed to claim orphan attachment");
                            continue;
                        }
                    };
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
                for key in keys {
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
