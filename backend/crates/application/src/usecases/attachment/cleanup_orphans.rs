use std::sync::Arc;

use domain::attachment::repository::AttachmentRepository;

use crate::ports::file_storage::{FileStorage, FileStorageError};

pub struct CleanupOrphanAttachments<R: AttachmentRepository, S: FileStorage> {
    repository: Arc<R>,
    storage: Arc<S>,
}

impl<R: AttachmentRepository, S: FileStorage> CleanupOrphanAttachments<R, S> {
    pub fn new(repository: Arc<R>, storage: Arc<S>) -> Self {
        Self {
            repository,
            storage,
        }
    }

    pub async fn run_once(&self) {
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
    }
}
