use async_trait::async_trait;
use domain::{attachment::Attachment, user::id::UserId};
use ports::repository::RepositoryError;
use media_domain::attachment::repository::AttachmentRepository;
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

fn row_to_attachment(
    id: Uuid,
    owner_id: Uuid,
    file_name: String,
    content_type: String,
    size_bytes: i64,
    storage_key: String,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Attachment {
    Attachment {
        id,
        owner_id,
        file_name,
        content_type,
        size_bytes,
        storage_key,
        created_at,
    }
}

#[async_trait]
impl AttachmentRepository for PostgresAttachmentRepository {
    async fn insert_reserving_quota(
        &self,
        a: &Attachment,
        max_count: i64,
        max_bytes: i64,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"SELECT pg_advisory_xact_lock(hashtextextended($1, 0))"#,
            a.owner_id.to_string(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let usage = sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!", COALESCE(SUM(size_bytes), 0)::BIGINT AS "bytes!"
            FROM attachments
            WHERE owner_id = $1 AND claimed_at IS NULL
              AND deleting_at IS NULL
            "#,
            a.owner_id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if usage.count >= max_count || usage.bytes + a.size_bytes > max_bytes {
            return Ok(false);
        }

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
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn set_size(&self, id: Uuid, size_bytes: i64) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"UPDATE attachments SET size_bytes = $2 WHERE id = $1"#,
            id,
            size_bytes,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Attachment>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT id, owner_id, file_name, content_type, size_bytes, storage_key, created_at
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
                r.created_at,
            )
        }))
    }

    async fn find_unclaimed_owned(
        &self,
        id: Uuid,
        owner_id: &UserId,
    ) -> Result<Option<Attachment>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT id, owner_id, file_name, content_type, size_bytes, storage_key, created_at
            FROM attachments
            WHERE id = $1 AND owner_id = $2
              AND claimed_at IS NULL AND deleting_at IS NULL
            "#,
            id,
            owner_id.uuid(),
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
                r.created_at,
            )
        }))
    }

    async fn claim_owned(
        &self,
        ids: &[Uuid],
        owner_id: &UserId,
        claim_token: Uuid,
    ) -> Result<Vec<Attachment>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            UPDATE attachments
            SET claimed_at = now(), claim_token = $3
            WHERE id = ANY($1) AND owner_id = $2
              AND claimed_at IS NULL AND deleting_at IS NULL
            RETURNING id, owner_id, file_name, content_type, size_bytes, storage_key, created_at
            "#,
            ids,
            owner_id.uuid(),
            claim_token,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let distinct: std::collections::HashSet<&Uuid> = ids.iter().collect();
        if rows.len() != distinct.len() {
            let claimed: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
            if !claimed.is_empty() {
                let _ = self.release_claims_by_token(&claimed, claim_token).await;
            }
            return Err(RepositoryError::InvalidData(
                "attachments unavailable".to_string(),
            ));
        }

        let mut attachments: Vec<Attachment> = rows
            .into_iter()
            .map(|r| {
                row_to_attachment(
                    r.id,
                    r.owner_id,
                    r.file_name,
                    r.content_type,
                    r.size_bytes,
                    r.storage_key,
                    r.created_at,
                )
            })
            .collect();
        attachments.sort_by_key(|a| a.created_at);
        Ok(attachments)
    }

    async fn release_claims(&self, ids: &[Uuid]) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            UPDATE attachments
            SET claimed_at = NULL, claim_token = NULL
            WHERE id = ANY($1)
            "#,
            ids,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn release_claims_by_token(
        &self,
        ids: &[Uuid],
        claim_token: Uuid,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            UPDATE attachments
            SET claimed_at = NULL, claim_token = NULL
            WHERE id = ANY($1) AND claim_token = $2
            "#,
            ids,
            claim_token,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn delete_by_storage_keys(&self, keys: &[String]) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"DELETE FROM attachments WHERE storage_key = ANY($1)"#,
            keys,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn claim_unlinked(
        &self,
        id: Uuid,
        owner_id: &UserId,
    ) -> Result<Option<String>, RepositoryError> {
        let row = sqlx::query_scalar!(
            r#"
            UPDATE attachments
            SET deleting_at = now()
            WHERE id = $1 AND owner_id = $2
              AND claimed_at IS NULL
            RETURNING storage_key
            "#,
            id,
            owner_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row)
    }

    async fn enqueue_cleanup(&self, keys: &[String]) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO storage_cleanup_queue (storage_key)
            SELECT * FROM UNNEST($1::text[])
            ON CONFLICT (storage_key) DO NOTHING
            "#,
            keys,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn list_stale_claims(
        &self,
        hours: i64,
        limit: i64,
        after: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
    ) -> Result<Vec<(Uuid, chrono::DateTime<chrono::Utc>)>, RepositoryError> {
        let (after_at, after_id) = match after {
            Some((at, id)) => (Some(at), Some(id)),
            None => (None, None),
        };
        let rows = sqlx::query!(
            r#"
            SELECT id, claimed_at AS "claimed_at!"
            FROM attachments
            WHERE claimed_at IS NOT NULL
              AND deleting_at IS NULL
              AND claimed_at < now() - ($1 || ' hours')::interval
              AND ($3::timestamptz IS NULL OR (claimed_at, id) > ($3, $4))
            ORDER BY claimed_at ASC, id ASC
            LIMIT $2
            "#,
            hours.to_string(),
            limit,
            after_at,
            after_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|r| (r.id, r.claimed_at)).collect())
    }

    async fn list_cleanup_queue(&self, limit: i64) -> Result<Vec<String>, RepositoryError> {
        let rows = sqlx::query_scalar!(
            r#"
            SELECT storage_key FROM storage_cleanup_queue
            ORDER BY created_at ASC
            LIMIT $1
            "#,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows)
    }

    async fn dequeue_cleanup(&self, key: &str) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"DELETE FROM storage_cleanup_queue WHERE storage_key = $1"#,
            key,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn restore_claims(&self, ids: &[Uuid]) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            UPDATE attachments
            SET claimed_at = now(), deleting_at = NULL
            WHERE id = ANY($1) AND claimed_at IS NULL
            "#,
            ids,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn list_orphans_older_than(
        &self,
        hours: i64,
        limit: i64,
    ) -> Result<Vec<Uuid>, RepositoryError> {
        let rows = sqlx::query_scalar!(
            r#"
            SELECT id
            FROM attachments
            WHERE claimed_at IS NULL
              AND (deleting_at IS NULL OR deleting_at < now() - interval '1 hour')
              AND created_at < now() - ($1 || ' hours')::interval
            ORDER BY created_at ASC
            LIMIT $2
            "#,
            hours.to_string(),
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows)
    }

    async fn claim_orphan(&self, id: Uuid) -> Result<Option<String>, RepositoryError> {
        let row = sqlx::query_scalar!(
            r#"
            UPDATE attachments
            SET deleting_at = now()
            WHERE id = $1 AND claimed_at IS NULL
              AND (deleting_at IS NULL OR deleting_at < now() - interval '1 hour')
            RETURNING storage_key
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row)
    }

    async fn delete_orphan_row(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            DELETE FROM attachments
            WHERE id = $1 AND claimed_at IS NULL
            "#,
            id,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
