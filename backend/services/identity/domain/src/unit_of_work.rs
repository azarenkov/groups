use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::{events::IdentityEvent, refresh_token::RefreshToken, refresh_token::id::RefreshTokenId, user::User, verification::code::VerificationCode};

#[async_trait::async_trait]
pub trait IdentityUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn IdentityTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait IdentityTransaction: Send {
    async fn load_user_for_update(&mut self, user_id: &UserId) -> Result<Option<User>, RepositoryError>;
    async fn load_pending_verification_for_update(&mut self, user_id: &UserId) -> Result<Option<VerificationCode>, RepositoryError>;
    async fn load_refresh_token_for_update(&mut self, id: &RefreshTokenId) -> Result<Option<RefreshToken>, RepositoryError>;
    async fn insert_user(&mut self, user: &User) -> Result<(), RepositoryError>;
    async fn save_user(&mut self, user: &User) -> Result<(), RepositoryError>;
    async fn insert_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError>;
    async fn save_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError>;
    async fn insert_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError>;
    async fn save_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError>;
    async fn save_events(&mut self, events: &[IdentityEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
