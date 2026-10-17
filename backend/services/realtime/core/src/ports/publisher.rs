use async_trait::async_trait;
use realtime_domain::envelope::Envelope;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("serialize envelope: {0}")]
    Serialize(serde_json::Error),
}

#[async_trait]
pub trait RealtimePublisher: Send + Sync {
    async fn publish_to_users(&self, recipients: Vec<Uuid>, envelope: &Envelope<'_>) -> Result<(), PublishError>;
    async fn announce_presence_preference(&self, user_id: Uuid, appear_offline: bool) -> Result<(), PublishError>;
}
