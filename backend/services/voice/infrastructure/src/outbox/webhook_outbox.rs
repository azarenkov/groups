use async_trait::async_trait;
use ports::{outbox_writer::OutboxWriter, repository::RepositoryError};
use voice::ports::webhook_outbox::WebhookOutbox;
use voice_domain::webhook::WebhookEvent;

use crate::{database::postgres::repositories::outbox_writer::PostgresOutboxWriter, outbox::mapping::to_record};

pub struct PostgresWebhookOutbox {
    writer: PostgresOutboxWriter,
}

impl PostgresWebhookOutbox {
    pub fn new(writer: PostgresOutboxWriter) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl WebhookOutbox for PostgresWebhookOutbox {
    async fn save(&self, event: &WebhookEvent, request_id: Option<&str>) -> Result<(), RepositoryError> {
        let record = to_record(event, request_id)?;
        self.writer.save(&[record]).await
    }
}
