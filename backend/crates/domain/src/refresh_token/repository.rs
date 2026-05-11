use async_trait::async_trait;

use crate::{
    refresh_token::{id::RefreshTokenId, refresh_token::RefreshToken},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

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
    async fn revoke(&self, refresh_token: &RefreshToken) -> Result<(), RepositoryError>;
    async fn revoke_all_for_user(&self, user_id: &UserId) -> Result<(), RepositoryError>;
}
