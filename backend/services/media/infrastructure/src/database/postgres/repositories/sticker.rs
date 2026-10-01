use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use media_domain::sticker::{
    AddStickerOutcome, InstallOutcome, MAX_INSTALLED_PACKS, MAX_STICKERS_PER_PACK, NewSticker, Sticker,
    StickerPack, StickerRef, StickerRepository, escape_like,
};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

const SEARCH_PREVIEW: i64 = 5;
const MAX_OWNED_PACKS_CAP: i64 = 50;

pub struct PostgresStickerRepository {
    pool: Pool<Postgres>,
}

impl PostgresStickerRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    async fn stickers_for(
        &self,
        pack_ids: &[Uuid],
        per_pack: Option<i64>,
    ) -> Result<HashMap<Uuid, Vec<Sticker>>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT id AS "id!", pack_id AS "pack_id!", storage_key AS "storage_key!", animated AS "animated!",
                   width AS "width!", height AS "height!", emoji AS "emoji!", position AS "position!"
            FROM (
                SELECT s.*, row_number() OVER (PARTITION BY s.pack_id ORDER BY s.position, s.created_at) AS rank
                FROM media_stickers s
                WHERE s.pack_id = ANY($1) AND s.deleted_at IS NULL
            ) ranked
            WHERE $2::bigint IS NULL OR rank <= $2
            ORDER BY pack_id, position, created_at
            "#,
            pack_ids,
            per_pack,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let mut by_pack: HashMap<Uuid, Vec<Sticker>> = HashMap::new();
        for r in rows {
            by_pack.entry(r.pack_id).or_default().push(Sticker {
                id: r.id,
                pack_id: r.pack_id,
                storage_key: r.storage_key,
                animated: r.animated,
                width: r.width,
                height: r.height,
                emoji: r.emoji,
                position: r.position,
            });
        }
        Ok(by_pack)
    }

    async fn with_stickers(
        &self,
        packs: Vec<PackRow>,
        per_pack: Option<i64>,
    ) -> Result<Vec<StickerPack>, RepositoryError> {
        let ids: Vec<Uuid> = packs.iter().map(|p| p.id).collect();
        let mut stickers = self.stickers_for(&ids, per_pack).await?;
        Ok(packs
            .into_iter()
            .map(|p| StickerPack {
                stickers: stickers.remove(&p.id).unwrap_or_default(),
                id: p.id,
                owner_id: UserId::new(p.owner_id),
                name: p.name,
                slug: p.slug,
                install_count: p.install_count,
                sticker_count: p.sticker_count,
                created_at: p.created_at,
            })
            .collect())
    }
}

struct PackRow {
    id: Uuid,
    owner_id: Uuid,
    name: String,
    slug: String,
    install_count: i32,
    sticker_count: i64,
    created_at: DateTime<Utc>,
}

#[async_trait]
impl StickerRepository for PostgresStickerRepository {
    async fn count_owned_packs(&self, owner_id: &UserId) -> Result<i64, RepositoryError> {
        sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!" FROM media_sticker_packs WHERE owner_id = $1 AND deleted_at IS NULL"#,
            owner_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn create_pack(
        &self,
        id: Uuid,
        owner_id: &UserId,
        name: &str,
        slug: &str,
        max_owned: i64,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 46))",
            owner_id.uuid().to_string()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let owned = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!" FROM media_sticker_packs WHERE owner_id = $1 AND deleted_at IS NULL"#,
            owner_id.uuid(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if owned >= max_owned {
            return Ok(false);
        }
        sqlx::query!(
            r#"INSERT INTO media_sticker_packs (id, owner_id, name, slug, install_count) VALUES ($1, $2, $3, $4, 1)"#,
            id,
            owner_id.uuid(),
            name,
            slug,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"INSERT INTO media_sticker_installs (user_id, pack_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"#,
            owner_id.uuid(),
            id,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn find_pack(&self, id_or_slug: &str) -> Result<Option<StickerPack>, RepositoryError> {
        let id = Uuid::parse_str(id_or_slug).ok();
        let row = sqlx::query_as!(
            PackRow,
            r#"
            SELECT p.id, p.owner_id, p.name, p.slug, p.install_count, p.created_at,
                   (SELECT COUNT(*) FROM media_stickers s WHERE s.pack_id = p.id AND s.deleted_at IS NULL) AS "sticker_count!"
            FROM media_sticker_packs p
            WHERE p.deleted_at IS NULL AND (p.id = $1 OR p.slug = $2)
            "#,
            id,
            id_or_slug.to_lowercase(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(match row {
            Some(row) => self.with_stickers(vec![row], None).await?.pop(),
            None => None,
        })
    }

    async fn rename_pack(&self, pack_id: Uuid, owner_id: &UserId, name: &str) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            UPDATE media_sticker_packs SET name = $3, updated_at = now()
            WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
            "#,
            pack_id,
            owner_id.uuid(),
            name,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_pack(&self, pack_id: Uuid, owner_id: &UserId) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
            UPDATE media_sticker_packs SET deleted_at = now(), updated_at = now(), install_count = 0
            WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
            "#,
            pack_id,
            owner_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        sqlx::query!(r#"DELETE FROM media_sticker_installs WHERE pack_id = $1"#, pack_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn add_sticker(
        &self,
        pack_id: Uuid,
        owner_id: &UserId,
        sticker: &NewSticker,
    ) -> Result<AddStickerOutcome, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let owned = sqlx::query_scalar!(
            r#"
            SELECT id FROM media_sticker_packs
            WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
            FOR UPDATE
            "#,
            pack_id,
            owner_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if owned.is_none() {
            return Ok(AddStickerOutcome::PackNotFound);
        }
        let stats = sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!", COALESCE(MAX(position) + 1, 0) AS "next_position!"
            FROM media_stickers WHERE pack_id = $1 AND deleted_at IS NULL
            "#,
            pack_id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if stats.count >= MAX_STICKERS_PER_PACK {
            return Ok(AddStickerOutcome::PackFull);
        }
        sqlx::query!(
            r#"
            INSERT INTO media_stickers (id, pack_id, storage_key, animated, width, height, emoji, position)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            sticker.id,
            pack_id,
            sticker.storage_key,
            sticker.animated,
            sticker.width,
            sticker.height,
            sticker.emoji,
            stats.next_position,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(r#"UPDATE media_sticker_packs SET updated_at = now() WHERE id = $1"#, pack_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(AddStickerOutcome::Added)
    }

    async fn delete_sticker(&self, pack_id: Uuid, sticker_id: Uuid, owner_id: &UserId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            UPDATE media_stickers s SET deleted_at = now()
            FROM media_sticker_packs p
            WHERE s.id = $1 AND s.pack_id = $2 AND s.deleted_at IS NULL
              AND p.id = s.pack_id AND p.owner_id = $3 AND p.deleted_at IS NULL
            "#,
            sticker_id,
            pack_id,
            owner_id.uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn reorder_stickers(&self, pack_id: Uuid, owner_id: &UserId, order: &[Uuid]) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let owned = sqlx::query_scalar!(
            r#"
            SELECT id FROM media_sticker_packs
            WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL
            FOR UPDATE
            "#,
            pack_id,
            owner_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if owned.is_none() {
            return Ok(false);
        }
        sqlx::query!(
            r#"
            UPDATE media_stickers s SET position = (o.ord - 1)::integer
            FROM unnest($2::uuid[]) WITH ORDINALITY AS o(id, ord)
            WHERE s.id = o.id AND s.pack_id = $1 AND s.deleted_at IS NULL
            "#,
            pack_id,
            order,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn search_packs(&self, query: &str, limit: i64, offset: i64) -> Result<Vec<StickerPack>, RepositoryError> {
        let pattern = escape_like(query).to_lowercase();
        let rows = sqlx::query_as!(
            PackRow,
            r#"
            SELECT p.id, p.owner_id, p.name, p.slug, p.install_count, p.created_at,
                   (SELECT COUNT(*) FROM media_stickers s WHERE s.pack_id = p.id AND s.deleted_at IS NULL) AS "sticker_count!"
            FROM media_sticker_packs p
            WHERE p.deleted_at IS NULL AND NOT p.hidden
              AND EXISTS (SELECT 1 FROM media_stickers s WHERE s.pack_id = p.id AND s.deleted_at IS NULL)
              AND ($1 = '' OR lower(p.name) LIKE '%' || $1 || '%' ESCAPE '\' OR p.slug LIKE '%' || $1 || '%' ESCAPE '\')
            ORDER BY (lower(p.name) LIKE $1 || '%' ESCAPE '\' OR p.slug LIKE $1 || '%' ESCAPE '\') DESC,
                     p.install_count DESC, p.created_at DESC, p.id
            LIMIT $2 OFFSET $3
            "#,
            pattern,
            limit,
            offset,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        self.with_stickers(rows, Some(SEARCH_PREVIEW)).await
    }

    async fn packs_of(&self, user_id: &UserId) -> Result<Vec<StickerPack>, RepositoryError> {
        let rows = sqlx::query_as!(
            PackRow,
            r#"
            SELECT p.id, p.owner_id, p.name, p.slug, p.install_count, p.created_at,
                   (SELECT COUNT(*) FROM media_stickers s WHERE s.pack_id = p.id AND s.deleted_at IS NULL) AS "sticker_count!"
            FROM media_sticker_packs p
            LEFT JOIN media_sticker_installs i ON i.pack_id = p.id AND i.user_id = $1
            WHERE p.deleted_at IS NULL AND (i.user_id IS NOT NULL OR p.owner_id = $1)
            ORDER BY COALESCE(i.installed_at, p.created_at), p.id
            LIMIT $2
            "#,
            user_id.uuid(),
            MAX_INSTALLED_PACKS + MAX_OWNED_PACKS_CAP,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        self.with_stickers(rows, None).await
    }

    async fn install(&self, user_id: &UserId, pack_id: Uuid) -> Result<InstallOutcome, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 47))",
            user_id.uuid().to_string()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let exists = sqlx::query_scalar!(
            r#"SELECT id FROM media_sticker_packs WHERE id = $1 AND deleted_at IS NULL FOR UPDATE"#,
            pack_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if exists.is_none() {
            return Ok(InstallOutcome::PackNotFound);
        }
        let installed = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "count!" FROM media_sticker_installs WHERE user_id = $1"#,
            user_id.uuid(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let inserted = if installed >= MAX_INSTALLED_PACKS {
            let already = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM media_sticker_installs WHERE user_id = $1 AND pack_id = $2) AS "exists!""#,
                user_id.uuid(),
                pack_id,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            if !already {
                return Ok(InstallOutcome::LimitReached);
            }
            0
        } else {
            sqlx::query!(
                r#"INSERT INTO media_sticker_installs (user_id, pack_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"#,
                user_id.uuid(),
                pack_id,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .rows_affected()
        };
        if inserted > 0 {
            sqlx::query!(
                r#"UPDATE media_sticker_packs SET install_count = install_count + 1 WHERE id = $1"#,
                pack_id,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(InstallOutcome::Installed)
    }

    async fn uninstall(&self, user_id: &UserId, pack_id: Uuid) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        let removed = sqlx::query!(
            r#"DELETE FROM media_sticker_installs WHERE user_id = $1 AND pack_id = $2"#,
            user_id.uuid(),
            pack_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if removed.rows_affected() > 0 {
            sqlx::query!(
                r#"UPDATE media_sticker_packs SET install_count = GREATEST(install_count - 1, 0) WHERE id = $1"#,
                pack_id,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(removed.rows_affected() > 0)
    }

    async fn find_sticker(&self, sticker_id: Uuid) -> Result<Option<StickerRef>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT s.id, s.pack_id, s.storage_key, s.animated, s.width, s.height, s.emoji, s.position,
                   p.name AS pack_name, p.slug AS pack_slug,
                   (s.deleted_at IS NOT NULL OR p.deleted_at IS NOT NULL) AS "deleted!"
            FROM media_stickers s
            JOIN media_sticker_packs p ON p.id = s.pack_id
            WHERE s.id = $1
            "#,
            sticker_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| StickerRef {
            sticker: Sticker {
                id: r.id,
                pack_id: r.pack_id,
                storage_key: r.storage_key,
                animated: r.animated,
                width: r.width,
                height: r.height,
                emoji: r.emoji,
                position: r.position,
            },
            pack_name: r.pack_name,
            pack_slug: r.pack_slug,
            deleted: r.deleted,
        }))
    }

    async fn forget_user(&self, user_id: &UserId) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            UPDATE media_sticker_packs p SET install_count = GREATEST(p.install_count - 1, 0)
            FROM media_sticker_installs i
            WHERE i.user_id = $1 AND i.pack_id = p.id
            "#,
            user_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(r#"DELETE FROM media_sticker_installs WHERE user_id = $1"#, user_id.uuid())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            DELETE FROM media_sticker_installs
            WHERE pack_id IN (SELECT id FROM media_sticker_packs WHERE owner_id = $1 AND deleted_at IS NULL)
            "#,
            user_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            UPDATE media_sticker_packs SET deleted_at = now(), updated_at = now(), install_count = 0
            WHERE owner_id = $1 AND deleted_at IS NULL
            "#,
            user_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
