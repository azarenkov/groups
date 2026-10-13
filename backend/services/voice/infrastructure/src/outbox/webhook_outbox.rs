use async_trait::async_trait;
use ports::{outbox_writer::OutboxWriter, repository::RepositoryError};
use voice_domain::stream::StreamLimited;
use voice::ports::webhook_outbox::WebhookOutbox;
use voice_domain::webhook::WebhookEvent;

use crate::{
    database::postgres::repositories::outbox_writer::PostgresOutboxWriter,
    outbox::mapping::{stream_limited_record, to_record},
};

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

    async fn record_stream_limited(&self, event: &StreamLimited) -> Result<(), RepositoryError> {
        self.writer.save(&[stream_limited_record(event)]).await
    }
}
