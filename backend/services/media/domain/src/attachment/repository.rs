use async_trait::async_trait;
use uuid::Uuid;

use crate::attachment::Attachment;
use domain::{shared::repository::error::RepositoryError, user::id::UserId};

#[async_trait]
pub trait AttachmentRepository {
    async fn insert_reserving_quota(
        &self,
        attachment: &Attachment,
        max_count: i64,
        max_bytes: i64,
    ) -> Result<bool, RepositoryError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Attachment>, RepositoryError>;

    async fn find_unclaimed_owned(
        &self,
        id: Uuid,
        owner_id: &UserId,
    ) -> Result<Option<Attachment>, RepositoryError>;

    async fn claim_owned(
        &self,
        ids: &[Uuid],
        owner_id: &UserId,
        claim_token: Uuid,
    ) -> Result<Vec<Attachment>, RepositoryError>;

    async fn release_claims(&self, ids: &[Uuid]) -> Result<(), RepositoryError>;

    async fn release_claims_by_token(
        &self,
        ids: &[Uuid],
        claim_token: Uuid,
    ) -> Result<(), RepositoryError>;

    async fn list_stale_claims(
        &self,
        hours: i64,
        limit: i64,
        after: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
    ) -> Result<Vec<(Uuid, chrono::DateTime<chrono::Utc>)>, RepositoryError>;

    async fn delete_by_storage_keys(&self, keys: &[String]) -> Result<(), RepositoryError>;

    async fn claim_unlinked(
        &self,
        id: Uuid,
        owner_id: &UserId,
    ) -> Result<Option<String>, RepositoryError>;

    async fn enqueue_cleanup(&self, keys: &[String]) -> Result<(), RepositoryError>;

    async fn list_cleanup_queue(&self, limit: i64) -> Result<Vec<String>, RepositoryError>;

    async fn dequeue_cleanup(&self, key: &str) -> Result<(), RepositoryError>;

    async fn restore_claims(&self, ids: &[Uuid]) -> Result<(), RepositoryError>;

    async fn list_orphans_older_than(
        &self,
        hours: i64,
        limit: i64,
    ) -> Result<Vec<Uuid>, RepositoryError>;

    async fn claim_orphan(&self, id: Uuid) -> Result<Option<String>, RepositoryError>;

    async fn delete_orphan_row(&self, id: Uuid) -> Result<(), RepositoryError>;
}
