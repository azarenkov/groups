use std::sync::Arc;

use chrono::Utc;
use domain::{
    refresh_token::{
        refresh_token::RefreshToken, repository::RefreshTokenRepository,
    },
    shared::repository::error::RepositoryError,
};

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
}

pub struct RefreshTokens<R, I>
where
    R: RefreshTokenRepository,
    I: TokenIssuer,
{
    refresh_token_repository: Arc<R>,
    token_issuer: Arc<I>,
}

impl<R, I> RefreshTokens<R, I>
where
    R: RefreshTokenRepository,
    I: TokenIssuer,
{
    pub fn new(refresh_token_repository: Arc<R>, token_issuer: Arc<I>) -> Self {
        Self {
            refresh_token_repository,
            token_issuer,
        }
    }

    pub async fn execute(
        &self,
        request: RefreshTokensRequest,
    ) -> Result<IssuedTokens, RefreshTokensError> {
        let decoded = self.token_issuer.decode_refresh(&request.refresh_token)?;

        let mut refresh_token = self
            .refresh_token_repository
            .find_by_id(&decoded.refresh_token_id)
            .await?
            .ok_or(RefreshTokensError::RefreshTokenNotFound)?;

        if refresh_token.user_id().uuid() != decoded.user_id.uuid() {
            return Err(RefreshTokensError::UserMismatch);
        }

        let now = Utc::now();

        if refresh_token.is_revoked() {
            return Err(RefreshTokensError::RefreshTokenRevoked);
        }

        if refresh_token.is_expired(now) {
            return Err(RefreshTokensError::RefreshTokenExpired);
        }

        refresh_token.revoke(now);
        let revoked = self
            .refresh_token_repository
            .revoke(&refresh_token)
            .await?;

        if !revoked {
            return Err(RefreshTokensError::RefreshTokenRevoked);
        }

        let tokens = self.token_issuer.issue(&decoded.user_id).await?;

        let new_refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            decoded.user_id,
            tokens.refresh_token_expires_at,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
        );
        self.refresh_token_repository
            .save_refresh_token(&new_refresh_token)
            .await?;

        Ok(tokens)
    }
}
