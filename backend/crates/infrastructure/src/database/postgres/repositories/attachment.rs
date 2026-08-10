use async_trait::async_trait;
use domain::{
    attachment::{Attachment, repository::AttachmentRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresAttachmentRepository {
    pool: Pool<Postgres>,
}

impl PostgresAttachmentRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[allow(clippy::too_many_arguments)]
fn row_to_attachment(
    id: Uuid,
    owner_id: Uuid,
    file_name: String,
    content_type: String,
    size_bytes: i64,
    storage_key: String,
    message_id: Option<Uuid>,
    dm_message_id: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Attachment {
    Attachment {
        id,
        owner_id,
        file_name,
        content_type,
        size_bytes,
        storage_key,
        message_id,
        dm_message_id,
        created_at,
    }
}

#[async_trait]
impl AttachmentRepository for PostgresAttachmentRepository {
    async fn insert(&self, a: &Attachment) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO attachments
                (id, owner_id, file_name, content_type, size_bytes, storage_key, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            a.id,
            a.owner_id,
            a.file_name,
            a.content_type,
            a.size_bytes,
            a.storage_key,
            a.created_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Attachment>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT id, owner_id, file_name, content_type, size_bytes, storage_key,
                   message_id, dm_message_id, created_at
            FROM attachments
            WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(row.map(|r| {
            row_to_attachment(
                r.id,
                r.owner_id,
                r.file_name,
                r.content_type,
                r.size_bytes,
                r.storage_key,
                r.message_id,
                r.dm_message_id,
                r.created_at,
            )
        }))
    }

    async fn list_for_messages(
        &self,
        message_ids: &[Uuid],
    ) -> Result<Vec<Attachment>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, owner_id, file_name, content_type, size_bytes, storage_key,
                   message_id, dm_message_id, created_at
            FROM attachments
            WHERE message_id = ANY($1)
            ORDER BY created_at ASC
            "#,
            message_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                row_to_attachment(
                    r.id,
                    r.owner_id,
                    r.file_name,
                    r.content_type,
                    r.size_bytes,
                    r.storage_key,
                    r.message_id,
                    r.dm_message_id,
                    r.created_at,
                )
            })
            .collect())
    }

    async fn can_access(
        &self,
        attachment_id: Uuid,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM attachments a
                LEFT JOIN messages m ON m.id = a.message_id
                LEFT JOIN channel_text_participants p
                    ON p.channel_id = m.channel_id AND p.user_id = $2
                LEFT JOIN dm_messages d ON d.id = a.dm_message_id
                WHERE a.id = $1
                  AND (
                    a.owner_id = $2
                    OR p.user_id IS NOT NULL
                    OR d.sender_id = $2
                    OR d.recipient_id = $2
                  )
            ) AS "exists!"
            "#,
            attachment_id,
            user_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.exists)
    }

    async fn count_unlinked_owned(
        &self,
        ids: &[Uuid],
        owner_id: &UserId,
    ) -> Result<i64, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!"
            FROM attachments
            WHERE id = ANY($1) AND owner_id = $2
              AND message_id IS NULL AND dm_message_id IS NULL
            "#,
            ids,
            owner_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.count)
    }

    async fn list_for_dm_messages(
        &self,
        dm_message_ids: &[Uuid],
    ) -> Result<Vec<Attachment>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, owner_id, file_name, content_type, size_bytes, storage_key,
                   message_id, dm_message_id, created_at
            FROM attachments
            WHERE dm_message_id = ANY($1)
            ORDER BY created_at ASC
            "#,
            dm_message_ids,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                row_to_attachment(
                    r.id,
                    r.owner_id,
                    r.file_name,
                    r.content_type,
                    r.size_bytes,
                    r.storage_key,
                    r.message_id,
                    r.dm_message_id,
                    r.created_at,
                )
            })
            .collect())
    }
}
