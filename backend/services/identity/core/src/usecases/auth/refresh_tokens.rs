use std::sync::Arc;

use chrono::Utc;
use identity_domain::{refresh_token::RefreshToken, refresh_token::RefreshTokenError, unit_of_work::IdentityUnitOfWork, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::{
    dto::requests::auth::refresh_tokens::RefreshTokensRequest,
    ports::token_issuer::{IssuedTokens, TokenIssuer, TokenIssuerError},
};

#[derive(Debug, thiserror::Error)]
pub enum RefreshTokensError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
    #[error("Refresh token not found")]
    RefreshTokenNotFound,
    #[error("Refresh token has been revoked")]
    RefreshTokenRevoked,
    #[error("Refresh token has expired")]
    RefreshTokenExpired,
    #[error("Refresh token does not belong to claimed user")]
    UserMismatch,
    #[error("User not found")]
    UserNotFound,
}

impl From<RefreshTokenError> for RefreshTokensError {
    fn from(err: RefreshTokenError) -> Self {
        match err {
            RefreshTokenError::ForeignToken => Self::UserMismatch,
            RefreshTokenError::Revoked => Self::RefreshTokenRevoked,
            RefreshTokenError::Expired => Self::RefreshTokenExpired,
        }
    }
}

pub struct RefreshTokens<W, U, I>
where
    W: IdentityUnitOfWork,
    U: UserRepository,
    I: TokenIssuer,
{
    unit_of_work: Arc<W>,
    user_repository: Arc<U>,
    token_issuer: Arc<I>,
}

impl<W, U, I> RefreshTokens<W, U, I>
where
    W: IdentityUnitOfWork,
    U: UserRepository,
    I: TokenIssuer,
{
    pub fn new(unit_of_work: Arc<W>, user_repository: Arc<U>, token_issuer: Arc<I>) -> Self {
        Self {
            unit_of_work,
            user_repository,
            token_issuer,
        }
    }

    pub async fn execute(
        &self,
        request: RefreshTokensRequest,
    ) -> Result<IssuedTokens, RefreshTokensError> {
        let decoded = self.token_issuer.decode_refresh(&request.refresh_token)?;
        let user = self
            .user_repository
            .find_by_id(&decoded.user_id)
            .await?
            .ok_or(RefreshTokensError::UserNotFound)?;
        let tokens = self
            .token_issuer
            .issue(&decoded.user_id, user.is_verified())
            .await?;

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut refresh_token = tx
            .load_refresh_token_for_update(&decoded.refresh_token_id)
            .await?
            .ok_or(RefreshTokensError::RefreshTokenNotFound)?;
        refresh_token.rotate(&decoded.user_id, now)?;
        let new_refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            decoded.user_id,
            tokens.refresh_token_expires_at,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
        );
        tx.save_refresh_token(&refresh_token).await?;
        tx.insert_refresh_token(&new_refresh_token).await?;
        tx.commit().await?;

        Ok(tokens)
    }
}
