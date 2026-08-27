use async_trait::async_trait;

use crate::{refresh_token::RefreshToken, refresh_token::id::RefreshTokenId, user::id::UserId};
use ports::repository::RepositoryError;

#[async_trait]
pub trait RefreshTokenRepository {
    async fn save_refresh_token(
        &self,
        refresh_token: &RefreshToken,
    ) -> Result<(), RepositoryError>;
    async fn find_by_id(
        &self,
        id: &RefreshTokenId,
    ) -> Result<Option<RefreshToken>, RepositoryError>;
    async fn revoke(&self, refresh_token: &RefreshToken) -> Result<bool, RepositoryError>;
    async fn revoke_all_for_user(&self, user_id: &UserId) -> Result<(), RepositoryError>;
}
