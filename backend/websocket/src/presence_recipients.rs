use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PresenceRecipientResolver {
    pool: PgPool,
}

impl PresenceRecipientResolver {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn interested_in(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT user_id FROM (
                SELECT friend_id AS user_id
                FROM realtime_friend_edges
                WHERE user_id = $1
                UNION
                SELECT DISTINCT sm2.user_id AS user_id
                FROM realtime_server_members sm1
                INNER JOIN realtime_server_members sm2 ON sm2.server_id = sm1.server_id
                WHERE sm1.user_id = $1 AND sm2.user_id <> $1
            ) AS interested
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }
}
