use std::sync::Arc;

use domain::attachment::repository::AttachmentRepository;

use crate::ports::file_storage::FileStorage;

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
        match self.repository.take_orphans_older_than(24, 200).await {
            Ok(keys) => {
                for key in keys {
                    if let Err(e) = self.storage.delete(&key).await {
                        tracing::warn!(error = %e, %key, "Failed to delete orphan attachment object");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Orphan attachment cleanup query failed");
            }
        }
    }
}
