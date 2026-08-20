use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct RecipientResolver {
    pool: PgPool,
}

impl RecipientResolver {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn server_members(&self, server_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows =
            sqlx::query("SELECT user_id FROM realtime_server_members WHERE server_id = $1 AND NOT removed")
                .bind(server_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }

    pub async fn friends_and_server_peers(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT friend_id AS user_id
            FROM realtime_friend_edges
            WHERE user_id = $1 AND NOT removed AND NOT pending
            UNION
            SELECT DISTINCT sm2.user_id
            FROM realtime_server_members sm1
            INNER JOIN realtime_server_members sm2
                ON sm2.server_id = sm1.server_id AND NOT sm2.removed
            WHERE sm1.user_id = $1 AND NOT sm1.removed AND sm2.user_id <> $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }

    pub async fn user_name_versioned(
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

    pub async fn user_name(&self, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT name FROM realtime_user_names WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<String, _>(0)))
    }

    pub async fn upsert_user_name(
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

    pub async fn add_friend_edge(
        &self,
        a: Uuid,
        b: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        self.upsert_friend_edge(a, b, false, false, occurred_at).await
    }

    pub async fn add_pending_friend_edge(
        &self,
        a: Uuid,
        b: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        self.upsert_friend_edge(a, b, false, true, occurred_at).await
    }

    pub async fn remove_friend_edge(
        &self,
        a: Uuid,
        b: Uuid,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        self.upsert_friend_edge(a, b, true, false, occurred_at).await
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

    pub async fn rename_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT friend_id AS user_id
            FROM realtime_friend_edges
            WHERE user_id = $1 AND NOT removed
            UNION
            SELECT DISTINCT sm2.user_id
            FROM realtime_server_members sm1
            INNER JOIN realtime_server_members sm2
                ON sm2.server_id = sm1.server_id AND NOT sm2.removed
            WHERE sm1.user_id = $1 AND NOT sm1.removed AND sm2.user_id <> $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }

    pub async fn add_server_member(
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

    pub async fn remove_server_member(
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

    pub async fn remove_server(
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
}
