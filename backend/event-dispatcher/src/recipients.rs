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

    pub async fn user_name(&self, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT name FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<String, _>(0)))
    }

    pub async fn dm_call_for(
        &self,
        message_id: Uuid,
    ) -> Result<Option<(String, Option<i32>)>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT call_status, call_duration_seconds
            FROM dm_messages
            WHERE id = $1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let status: Option<String> = row.get(0);
        let duration: Option<i32> = row.get(1);
        Ok(status.map(|s| (s, duration)))
    }

    pub async fn dm_invite_for(
        &self,
        message_id: Uuid,
    ) -> Result<Option<(Uuid, String)>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT m.invite_server_id, s.name
            FROM dm_messages m
            LEFT JOIN servers s ON s.id = m.invite_server_id
            WHERE m.id = $1
            "#,
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let server_id: Option<Uuid> = row.get(0);
        let server_name: Option<String> = row.get(1);
        Ok(match (server_id, server_name) {
            (Some(id), Some(name)) => Some((id, name)),
            _ => None,
        })
    }

    pub async fn attachments_for_message(
        &self,
        message_id: Uuid,
        dm: bool,
    ) -> Result<Vec<serde_json::Value>, sqlx::Error> {
        let column = if dm { "dm_message_id" } else { "message_id" };
        let rows = sqlx::query(&format!(
            "SELECT id, file_name, content_type, size_bytes FROM attachments WHERE {column} = $1 ORDER BY created_at ASC"
        ))
        .bind(message_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.get::<Uuid, _>(0),
                    "fileName": r.get::<String, _>(1),
                    "contentType": r.get::<String, _>(2),
                    "sizeBytes": r.get::<i64, _>(3),
                })
            })
            .collect())
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
