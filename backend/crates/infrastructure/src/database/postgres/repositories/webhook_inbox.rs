use application::ports::webhook_inbox::{WebhookInbox, WebhookSource};
use async_trait::async_trait;
use domain::shared::repository::error::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::webhook_source::WebhookSourceDto, error::map_sqlx_error_to_domain_error,
};

pub struct PostgresWebhookInbox {
    pool: Pool<Postgres>,
}

impl PostgresWebhookInbox {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebhookInbox for PostgresWebhookInbox {
    async fn try_record_processed(
        &self,
        source: WebhookSource,
        external_id: &str,
    ) -> Result<bool, RepositoryError> {
        let source_dto = WebhookSourceDto::from(source);

        let result = sqlx::query!(
            r#"
                INSERT INTO webhook_inbox (source, external_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
            "#,
            source_dto as WebhookSourceDto,
            external_id,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.rows_affected() > 0)
    }
}
