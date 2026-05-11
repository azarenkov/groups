use async_trait::async_trait;
use domain::shared::repository::error::RepositoryError;

#[derive(Debug, Clone, Copy)]
pub enum WebhookSource {
    LiveKit,
}

#[async_trait]
pub trait WebhookInbox: Send + Sync {
    async fn try_record_processed(
        &self,
        source: WebhookSource,
        external_id: &str,
    ) -> Result<bool, RepositoryError>;
}
