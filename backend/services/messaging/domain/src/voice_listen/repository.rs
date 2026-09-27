use async_trait::async_trait;
use uuid::Uuid;

use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[async_trait]
pub trait VoiceListenRepository: Send + Sync {
    async fn mark_listened(
        &self,
        message_id: Uuid,
        listener: &UserId,
        notify: &[Uuid],
    ) -> Result<(), RepositoryError>;

    async fn listened_in_channel(
        &self,
        message_ids: &[Uuid],
        viewer: &UserId,
    ) -> Result<Vec<Uuid>, RepositoryError>;

    async fn listened_in_dm(
        &self,
        message_ids: &[Uuid],
        viewer: &UserId,
    ) -> Result<Vec<Uuid>, RepositoryError>;
}
