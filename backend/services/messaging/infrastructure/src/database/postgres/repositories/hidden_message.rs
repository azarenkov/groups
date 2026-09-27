use async_trait::async_trait;
use domain::user::id::UserId;
use messaging_domain::hidden_message::repository::HiddenMessageRepository;
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresHiddenMessageRepository {
    pool: Pool<Postgres>,
}

impl PostgresHiddenMessageRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HiddenMessageRepository for PostgresHiddenMessageRepository {
    async fn hide_for_user(&self, user_id: &UserId, message_id: Uuid) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let hidden_at = sqlx::query_scalar!(
            r#"
            INSERT INTO hidden_messages (user_id, message_id)
            VALUES ($1, $2)
            ON CONFLICT (user_id, message_id) DO UPDATE SET hidden_at = hidden_messages.hidden_at
            RETURNING hidden_at
            "#,
            user_id.uuid(),
            message_id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let record = OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: user_id.uuid(),
            aggregate_type: AggregateType::User,
            parent_id: None,
            parent_type: None,
            event_type: "message.hidden".to_string(),
            payload: serde_json::json!({
                "user_id": user_id.uuid(),
                "message_id": message_id,
                "hidden_at": hidden_at,
            }),
            occurred_at: hidden_at,
            request_id: None,
        };
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
