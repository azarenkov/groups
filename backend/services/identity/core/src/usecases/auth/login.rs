use std::sync::Arc;

use chrono::Utc;
use identity_domain::{refresh_token::RefreshToken, unit_of_work::IdentityUnitOfWork, user::email::Email, user::events::UserEvent, user::events::lifecycle::UserLoggedIn, user::repository::UserRepository};
use ports::{outbox::record::OutboxRecord, outbox::to_outbox::OutboxError, outbox::to_outbox::ToOutboxRecord, repository::RepositoryError};

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

pub struct LoginUser<T, H, W, I>
where
    T: UserRepository,
    H: Hasher,
    W: IdentityUnitOfWork,
    I: TokenIssuer,
{
    user_repository: Arc<T>,
    hasher: Arc<H>,
    unit_of_work: Arc<W>,
    token_issuer: Arc<I>,
}

impl<T, H, W, I> LoginUser<T, H, W, I>
where
    T: UserRepository,
    H: Hasher,
    W: IdentityUnitOfWork,
    I: TokenIssuer,
{
    pub fn new(
        user_repository: Arc<T>,
        hasher: Arc<H>,
        unit_of_work: Arc<W>,
        token_issuer: Arc<I>,
    ) -> Self {
        Self {
            user_repository,
            hasher,
            unit_of_work,
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
        let records: Vec<OutboxRecord> =
            vec![event.to_outbox_record_with_request_id(request.ctx.request_id)?];

        let mut tx = self.unit_of_work.begin().await?;
        tx.insert_refresh_token(&refresh_token).await?;
        tx.save_outbox(&records).await?;
        tx.commit().await?;

        Ok(LoginOutcome {
            tokens,
            requires_verification,
        })
    }
}
