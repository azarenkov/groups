use async_trait::async_trait;
use chrono::{DateTime, Utc};
use infrastructure_common::database::postgres::error::map_sqlx_error_to_domain_error;
use ports::repository::RepositoryError;
use realtime_core::ports::{directory::RecipientDirectory, projection::RealtimeProjection};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresRealtimeProjection {
    pool: PgPool,
}

impl PostgresRealtimeProjection {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn server_members(&self, server_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows =
            sqlx::query("SELECT user_id FROM realtime_server_members WHERE server_id = $1 AND NOT removed")
                .bind(server_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }

    async fn user_name_versioned(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(String, DateTime<Utc>)>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT name, updated_at FROM realtime_user_names WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| (r.get::<String, _>(0), r.get::<DateTime<Utc>, _>(1))))
    }

    async fn user_name(&self, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT name FROM realtime_user_names WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<String, _>(0)))
    }

    async fn upsert_user_name(
        &self,
        user_id: Uuid,
        name: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_user_names (user_id, name, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE
                SET name = EXCLUDED.name, updated_at = EXCLUDED.updated_at
                WHERE realtime_user_names.updated_at <= EXCLUDED.updated_at
            "#,
        )
        .bind(user_id)
        .bind(name)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn add_friend_edge(
        &self,
        a: Uuid,
        b: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        self.upsert_friend_edge(a, b, false, false, occurred_at).await
    }

    async fn add_pending_friend_edge(
        &self,
        a: Uuid,
        b: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        self.upsert_friend_edge(a, b, false, true, occurred_at).await
    }

    async fn remove_friend_edge(
        &self,
        a: Uuid,
        b: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        self.upsert_friend_edge(a, b, true, false, occurred_at).await
    }

    async fn apply_block(
        &self,
        blocker_id: Uuid,
        blocked_id: Uuid,
        active: bool,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_user_blocks (blocker_id, blocked_id, active, updated_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (blocker_id, blocked_id) DO UPDATE
                SET active = EXCLUDED.active,
                    updated_at = EXCLUDED.updated_at
                WHERE realtime_user_blocks.updated_at <= EXCLUDED.updated_at
            "#,
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .bind(active)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn record_dm_pair(&self, a: Uuid, b: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_dm_pairs (user_a, user_b)
            VALUES (LEAST($1, $2), GREATEST($1, $2))
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(a)
        .bind(b)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_friend_edge(
        &self,
        a: Uuid,
        b: Uuid,
        removed: bool,
        pending: bool,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_friend_edges (user_id, friend_id, removed, pending, updated_at)
            VALUES ($1, $2, $3, $4, $5), ($2, $1, $3, $4, $5)
            ON CONFLICT (user_id, friend_id) DO UPDATE
                SET removed = EXCLUDED.removed,
                    pending = EXCLUDED.pending,
                    updated_at = EXCLUDED.updated_at
                WHERE realtime_friend_edges.updated_at <= EXCLUDED.updated_at
            "#,
        )
        .bind(a)
        .bind(b)
        .bind(removed)
        .bind(pending)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn rename_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        self.related_recipients(user_id, false).await
    }

    async fn profile_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        self.related_recipients(user_id, true).await
    }

    async fn related_recipients(
        &self,
        user_id: Uuid,
        hide_from_blocked: bool,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT r.user_id
            FROM (
                SELECT e.friend_id AS user_id
                FROM realtime_friend_edges e
                WHERE e.user_id = $1
                  AND (
                    NOT e.removed
                    OR EXISTS (
                        SELECT 1 FROM realtime_dm_pairs p
                        WHERE p.user_a = LEAST($1, e.friend_id)
                          AND p.user_b = GREATEST($1, e.friend_id)
                    )
                  )
                UNION
                SELECT DISTINCT sm2.user_id
                FROM realtime_server_members sm1
                INNER JOIN realtime_server_members sm2
                    ON sm2.server_id = sm1.server_id AND NOT sm2.removed
                WHERE sm1.user_id = $1 AND NOT sm1.removed AND sm2.user_id <> $1
            ) r
            WHERE NOT (
                $2 AND EXISTS (
                    SELECT 1 FROM realtime_user_blocks b
                    WHERE b.active AND b.blocker_id = $1 AND b.blocked_id = r.user_id
                )
            )
            "#,
        )
        .bind(user_id)
        .bind(hide_from_blocked)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }

    async fn add_server_member(
        &self,
        server_id: Uuid,
        user_id: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_server_members (server_id, user_id, removed, updated_at)
            SELECT $1, $2, FALSE, $3
            WHERE NOT EXISTS (
                SELECT 1 FROM realtime_deleted_servers WHERE server_id = $1
            )
            ON CONFLICT (server_id, user_id) DO UPDATE
                SET removed = EXCLUDED.removed, updated_at = EXCLUDED.updated_at
                WHERE realtime_server_members.updated_at <= EXCLUDED.updated_at
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_server_member(
        &self,
        server_id: Uuid,
        user_id: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_server_members (server_id, user_id, removed, updated_at)
            VALUES ($1, $2, TRUE, $3)
            ON CONFLICT (server_id, user_id) DO UPDATE
                SET removed = EXCLUDED.removed, updated_at = EXCLUDED.updated_at
                WHERE realtime_server_members.updated_at <= EXCLUDED.updated_at
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_server(
        &self,
        server_id: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_deleted_servers (server_id, deleted_at)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(server_id)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            UPDATE realtime_server_members
            SET removed = TRUE, updated_at = $2
            WHERE server_id = $1 AND updated_at <= $2
            "#,
        )
        .bind(server_id)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    async fn interested_in_query(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT user_id FROM (
                SELECT friend_id AS user_id
                FROM realtime_friend_edges
                WHERE user_id = $1 AND NOT removed AND NOT pending
                UNION
                SELECT DISTINCT sm2.user_id AS user_id
                FROM realtime_server_members sm1
                INNER JOIN realtime_server_members sm2
                    ON sm2.server_id = sm1.server_id AND NOT sm2.removed
                WHERE sm1.user_id = $1 AND NOT sm1.removed AND sm2.user_id <> $1
            ) AS interested
            WHERE NOT EXISTS (
                SELECT 1 FROM realtime_user_blocks b
                WHERE b.active
                  AND (
                    (b.blocker_id = $1 AND b.blocked_id = interested.user_id)
                    OR (b.blocker_id = interested.user_id AND b.blocked_id = $1)
                  )
            )
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }
}

#[async_trait]
impl RealtimeProjection for PostgresRealtimeProjection {
    async fn upsert_user_name(&self, user_id: Uuid, name: &str, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::upsert_user_name(self, user_id, name, occurred_at).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn add_friend_edge(&self, a: Uuid, b: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::add_friend_edge(self, a, b, occurred_at).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn add_pending_friend_edge(&self, a: Uuid, b: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::add_pending_friend_edge(self, a, b, occurred_at).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn remove_friend_edge(&self, a: Uuid, b: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::remove_friend_edge(self, a, b, occurred_at).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn apply_block(&self, blocker_id: Uuid, blocked_id: Uuid, active: bool, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::apply_block(self, blocker_id, blocked_id, active, occurred_at).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn record_dm_pair(&self, a: Uuid, b: Uuid) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::record_dm_pair(self, a, b).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn add_server_member(&self, server_id: Uuid, user_id: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::add_server_member(self, server_id, user_id, occurred_at).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn remove_server_member(&self, server_id: Uuid, user_id: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::remove_server_member(self, server_id, user_id, occurred_at).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn remove_server(&self, server_id: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError> {
        PostgresRealtimeProjection::remove_server(self, server_id, occurred_at).await.map_err(map_sqlx_error_to_domain_error)
    }
}

#[async_trait]
impl RecipientDirectory for PostgresRealtimeProjection {
    async fn server_members(&self, server_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        PostgresRealtimeProjection::server_members(self, server_id).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn user_name(&self, user_id: Uuid) -> Result<Option<String>, RepositoryError> {
        PostgresRealtimeProjection::user_name(self, user_id).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn user_name_versioned(&self, user_id: Uuid) -> Result<Option<(String, DateTime<Utc>)>, RepositoryError> {
        PostgresRealtimeProjection::user_name_versioned(self, user_id).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn rename_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        PostgresRealtimeProjection::rename_recipients(self, user_id).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn profile_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        PostgresRealtimeProjection::profile_recipients(self, user_id).await.map_err(map_sqlx_error_to_domain_error)
    }
    async fn interested_in(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        self.interested_in_query(user_id).await.map_err(map_sqlx_error_to_domain_error)
    }
}
