use async_trait::async_trait;
use uuid::Uuid;

use crate::{attachment::Attachment, shared::repository::error::RepositoryError, user::id::UserId};

#[async_trait]
pub trait AttachmentRepository {
    async fn insert(&self, attachment: &Attachment) -> Result<(), RepositoryError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Attachment>, RepositoryError>;

    async fn list_for_messages(
        &self,
        message_ids: &[Uuid],
    ) -> Result<Vec<Attachment>, RepositoryError>;

    async fn list_for_dm_messages(
        &self,
        dm_message_ids: &[Uuid],
    ) -> Result<Vec<Attachment>, RepositoryError>;

    async fn delete_unlinked(
        &self,
        id: Uuid,
        owner_id: &UserId,
    ) -> Result<Option<String>, RepositoryError>;

    async fn storage_keys_for_message(
        &self,
        message_id: Uuid,
        dm: bool,
    ) -> Result<Vec<String>, RepositoryError>;

    async fn take_orphans_older_than(
        &self,
        hours: i64,
        limit: i64,
    ) -> Result<Vec<String>, RepositoryError>;

    async fn can_access(
        &self,
        attachment_id: Uuid,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError>;

    async fn count_unlinked_owned(
        &self,
        ids: &[Uuid],
        owner_id: &UserId,
    ) -> Result<i64, RepositoryError>;
}
