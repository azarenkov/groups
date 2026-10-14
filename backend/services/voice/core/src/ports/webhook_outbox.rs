use async_trait::async_trait;
use ports::repository::RepositoryError;
use voice_domain::stream::StreamLimited;
use voice_domain::webhook::WebhookEvent;

#[async_trait]
pub trait WebhookOutbox: Send + Sync {
    async fn save(&self, event: &WebhookEvent, request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn record_stream_limited(&self, event: &StreamLimited) -> Result<(), RepositoryError>;
}
