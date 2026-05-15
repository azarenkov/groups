use std::sync::Arc;

use chrono::Utc;
use domain::{
    refresh_token::{RefreshToken, repository::RefreshTokenRepository},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::{
        email::Email,
        events::{UserEvent, lifecycle::UserLoggedIn},
        repository::UserRepository,
    },
};

use crate::{
    dto::requests::auth::login_user::LoginUserRequest,
    ports::{
        hasher::{HashError, Hasher},
        token_issuer::{IssuedTokens, TokenIssuer, TokenIssuerError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
    #[error("Invalid credentials")]
    InvalidCredentials,
}

pub struct LoginOutcome {
    pub tokens: IssuedTokens,
    pub requires_verification: bool,
}

pub struct LoginUser<T, H, R, I>
where
    T: UserRepository,
    H: Hasher,
    R: RefreshTokenRepository,
    I: TokenIssuer,
{
    user_repository: Arc<T>,
    hasher: Arc<H>,
    refresh_token_repository: Arc<R>,
    token_issuer: Arc<I>,
}

impl<T, H, R, I> LoginUser<T, H, R, I>
where
    T: UserRepository,
    H: Hasher,
    R: RefreshTokenRepository,
    I: TokenIssuer,
{
    pub fn new(
        user_repository: Arc<T>,
        hasher: Arc<H>,
        refresh_token_repository: Arc<R>,
        token_issuer: Arc<I>,
    ) -> Self {
        Self {
            user_repository,
            hasher,
            refresh_token_repository,
            token_issuer,
        }
    }

    pub async fn execute(&self, request: LoginUserRequest) -> Result<LoginOutcome, LoginError> {
        let email = Email::new(request.email);
        let user = self
            .user_repository
            .find_by_email(&email)
            .await?
            .ok_or(LoginError::InvalidCredentials)?;

        if !self
            .hasher
            .verify(&request.password, user.password_hash().as_str())?
        {
            return Err(LoginError::InvalidCredentials);
        }

        let is_verified = user.is_verified();
        let requires_verification = !is_verified;
        let user_id = user.id_ownned();
        let tokens = self.token_issuer.issue(&user_id, is_verified).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user_id,
            tokens.refresh_token_expires_at,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
        );

        let event = UserEvent::LoggedIn(UserLoggedIn::record(
            user_id,
            tokens.refresh_token_id,
            request.client.ip,
            request.client.user_agent,
            Utc::now(),
        ));
        let request_id = shared::middleware::request_id::current();
        let records: Vec<OutboxRecord> = vec![event.to_outbox_record_with_request_id(request_id)?];

        self.refresh_token_repository
            .save_refresh_token_with_events(&refresh_token, &records)
            .await?;

        Ok(LoginOutcome {
            tokens,
            requires_verification,
        })
    }
}
