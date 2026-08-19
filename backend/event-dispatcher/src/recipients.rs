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
            sqlx::query("SELECT user_id FROM realtime_server_members WHERE server_id = $1")
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
            WHERE user_id = $1
            UNION
            SELECT DISTINCT sm2.user_id
            FROM realtime_server_members sm1
            INNER JOIN realtime_server_members sm2 ON sm2.server_id = sm1.server_id
            WHERE sm1.user_id = $1 AND sm2.user_id <> $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
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

    pub async fn add_friend_edge(&self, a: Uuid, b: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_friend_edges (user_id, friend_id)
            VALUES ($1, $2), ($2, $1)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(a)
        .bind(b)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_friend_edge(&self, a: Uuid, b: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM realtime_friend_edges
            WHERE (user_id = $1 AND friend_id = $2)
               OR (user_id = $2 AND friend_id = $1)
            "#,
        )
        .bind(a)
        .bind(b)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_server_member(
        &self,
        server_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO realtime_server_members (server_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(server_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_server_member(
        &self,
        server_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM realtime_server_members WHERE server_id = $1 AND user_id = $2",
        )
        .bind(server_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_server(&self, server_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM realtime_server_members WHERE server_id = $1")
            .bind(server_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
