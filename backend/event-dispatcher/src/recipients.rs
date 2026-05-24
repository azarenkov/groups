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
        let rows = sqlx::query("SELECT user_id FROM server_members WHERE server_id = $1")
            .bind(server_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }

    pub async fn text_channel_recipients(
        &self,
        channel_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT user_id FROM channel_text_participants WHERE channel_id = $1",
        )
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>(0)).collect())
    }

    pub async fn dm_recipient_for(
        &self,
        message_id: Uuid,
        sender_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT sender_id, recipient_id FROM dm_messages WHERE id = $1",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let s: Uuid = row.get(0);
        let r: Uuid = row.get(1);
        if s == sender_id {
            Ok(Some(r))
        } else if r == sender_id {
            Ok(Some(s))
        } else {
            Ok(Some(r))
        }
    }
}
