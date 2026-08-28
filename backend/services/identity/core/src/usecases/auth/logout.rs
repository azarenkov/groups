use std::sync::Arc;

use chrono::Utc;
use identity_domain::{events::IdentityEvent, refresh_token::RefreshTokenError, refresh_token::RevokeOutcome, unit_of_work::IdentityUnitOfWork, user::events::UserEvent, user::events::lifecycle::UserLoggedOut};
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

pub struct Logout<W, I>
where
    W: IdentityUnitOfWork,
    I: TokenIssuer,
{
    unit_of_work: Arc<W>,
    token_issuer: Arc<I>,
}

impl<W, I> Logout<W, I>
where
    W: IdentityUnitOfWork,
    I: TokenIssuer,
{
    pub fn new(unit_of_work: Arc<W>, token_issuer: Arc<I>) -> Self {
        Self {
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

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut refresh_token = tx
            .load_refresh_token_for_update(&decoded.refresh_token_id)
            .await?
            .ok_or(LogoutError::RefreshTokenNotFound)?;
        match refresh_token.revoke_by(&user_id, now) {
            Ok(RevokeOutcome::Revoked) => {}
            Ok(RevokeOutcome::AlreadyRevoked) => return Ok(()),
            Err(RefreshTokenError::ForeignToken) => return Err(LogoutError::UserMismatch),
            Err(RefreshTokenError::Revoked | RefreshTokenError::Expired) => return Ok(()),
        }

        let event = UserEvent::LoggedOut(UserLoggedOut::record(
            user_id,
            decoded.refresh_token_id,
            request.client.ip,
            request.client.user_agent,
            now,
        ));
        tx.save_refresh_token(&refresh_token).await?;
        tx.save_events(&[IdentityEvent::from(event)], request.ctx.request_id.as_deref())
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
