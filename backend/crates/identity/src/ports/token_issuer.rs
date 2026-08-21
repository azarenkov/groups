use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{refresh_token::id::RefreshTokenId, user::id::UserId};

#[derive(Debug)]
pub struct IssuedTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_token_id: RefreshTokenId,
    pub refresh_token_expires_at: DateTime<Utc>,
    pub access_token_expires_in: i64,
}

#[derive(Debug)]
pub struct DecodedRefreshToken {
    pub user_id: UserId,
    pub refresh_token_id: RefreshTokenId,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum TokenIssuerError {
    #[error("Token encoding error: {0}")]
    Encoding(String),
    #[error("Token decoding error: {0}")]
    Decoding(String),
    #[error("Invalid token type")]
    InvalidTokenType,
}

#[async_trait]
pub trait TokenIssuer: Send + Sync {
    async fn issue(
        &self,
        user_id: &UserId,
        verified: bool,
    ) -> Result<IssuedTokens, TokenIssuerError>;
    fn decode_refresh(&self, token: &str) -> Result<DecodedRefreshToken, TokenIssuerError>;
}
