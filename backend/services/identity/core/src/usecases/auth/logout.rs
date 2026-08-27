use std::sync::Arc;

use chrono::Utc;
use identity_domain::{events::IdentityEvent, refresh_token::repository::RefreshTokenRepository, unit_of_work::IdentityUnitOfWork, user::events::UserEvent, user::events::lifecycle::UserLoggedOut};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

use crate::{
    dto::requests::auth::logout::LogoutRequest,
    ports::token_issuer::{TokenIssuer, TokenIssuerError},
};

#[derive(Debug, thiserror::Error)]
pub enum LogoutError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
    #[error("Refresh token not found")]
    RefreshTokenNotFound,
    #[error("Refresh token does not belong to user")]
    UserMismatch,
}

pub struct Logout<R, W, I>
where
    R: RefreshTokenRepository,
    W: IdentityUnitOfWork,
    I: TokenIssuer,
{
    refresh_token_repository: Arc<R>,
    unit_of_work: Arc<W>,
    token_issuer: Arc<I>,
}

impl<R, W, I> Logout<R, W, I>
where
    R: RefreshTokenRepository,
    W: IdentityUnitOfWork,
    I: TokenIssuer,
{
    pub fn new(refresh_token_repository: Arc<R>, unit_of_work: Arc<W>, token_issuer: Arc<I>) -> Self {
        Self {
            refresh_token_repository,
            unit_of_work,
            token_issuer,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        request: LogoutRequest,
    ) -> Result<(), LogoutError> {
        let decoded = self.token_issuer.decode_refresh(&request.refresh_token)?;

        if decoded.user_id.uuid() != user_id.uuid() {
            return Err(LogoutError::UserMismatch);
        }

        let mut refresh_token = self
            .refresh_token_repository
            .find_by_id(&decoded.refresh_token_id)
            .await?
            .ok_or(LogoutError::RefreshTokenNotFound)?;

        if refresh_token.user_id().uuid() != user_id.uuid() {
            return Err(LogoutError::UserMismatch);
        }

        if refresh_token.is_revoked() {
            return Ok(());
        }

        let now = Utc::now();
        refresh_token.revoke(now);

        let event = UserEvent::LoggedOut(UserLoggedOut::record(
            user_id,
            decoded.refresh_token_id,
            request.client.ip,
            request.client.user_agent,
            now,
        ));
        let mut tx = self.unit_of_work.begin().await?;
        if !tx.revoke_refresh_token(&refresh_token).await? {
            return Ok(());
        }
        tx.save_events(&[IdentityEvent::from(event)], request.ctx.request_id.as_deref())
            .await?;
        tx.commit().await?;

        Ok(())
    }
}
