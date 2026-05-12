use async_trait::async_trait;

use crate::{
    refresh_token::{RefreshToken, id::RefreshTokenId},
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};

#[async_trait]
pub trait RefreshTokenRepository {
    async fn save_refresh_token(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<(), RepositoryError>;
    async fn save_refresh_token_with_events(
        &self,
        refresh_token: &RefreshToken,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;
    async fn find_by_id(
        &self,
        id: &RefreshTokenId,
    ) -> Result<Option<RefreshToken>, RepositoryError>;
    async fn revoke(&self, refresh_token: &RefreshToken) -> Result<bool, RepositoryError>;
    async fn revoke_with_events(
        &self,
        refresh_token: &RefreshToken,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;
    async fn revoke_all_for_user(&self, user_id: &UserId) -> Result<(), RepositoryError>;
}
