use domain::{user::id::UserId, user::name::UserName};
use ports::{outbox::record::OutboxRecord, repository::RepositoryError};

use crate::{refresh_token::RefreshToken, user::User, verification::code::VerificationCode};

#[async_trait::async_trait]
pub trait IdentityUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn IdentityTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait IdentityTransaction: Send {
    async fn insert_user(&mut self, user: &User) -> Result<(), RepositoryError>;
    async fn insert_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<(), RepositoryError>;
    async fn revoke_refresh_token(&mut self, refresh_token: &RefreshToken) -> Result<bool, RepositoryError>;
    async fn set_user_name(&mut self, user_id: &UserId, name: &UserName) -> Result<(), RepositoryError>;
    async fn insert_verification_code(&mut self, verification_code: &VerificationCode) -> Result<(), RepositoryError>;
    async fn mark_user_verified(&mut self, user: &User) -> Result<(), RepositoryError>;
    async fn consume_verification_code(&mut self, verification_code: &VerificationCode) -> Result<bool, RepositoryError>;
    async fn save_outbox(&mut self, records: &[OutboxRecord]) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
