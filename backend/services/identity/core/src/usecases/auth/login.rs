use std::sync::Arc;

use chrono::Utc;
use identity_domain::{events::IdentityEvent, refresh_token::RefreshToken, unit_of_work::IdentityUnitOfWork, user::email::Email, user::events::UserEvent, user::events::lifecycle::UserLoggedIn, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::{
    dto::requests::auth::login_user::LoginUserRequest,
    usecases::auth::reactivation::wait_past_cutoff,
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

        if user.is_deleted() {
            return Err(LoginError::InvalidCredentials);
        }

        if !self
            .hasher
            .verify(&request.password, user.password_hash().as_str())?
        {
            return Err(LoginError::InvalidCredentials);
        }

        let user_id = user.id_ownned();

        let mut tx = self.unit_of_work.begin().await?;
        let mut events = Vec::new();
        let mut locked = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(LoginError::InvalidCredentials)?;
        if locked.is_deleted() {
            return Err(LoginError::InvalidCredentials);
        }
        if locked.password_hash().as_str() != user.password_hash().as_str()
            && !self
                .hasher
                .verify(&request.password, locked.password_hash().as_str())?
        {
            return Err(LoginError::InvalidCredentials);
        }
        if let Some(disabled_at) = locked.disabled_at() {
            locked.reactivate(Utc::now());
            events.extend(locked.pull_events().into_iter().map(IdentityEvent::from));
            tx.save_user(&locked).await?;
            wait_past_cutoff(disabled_at).await;
        }
        if let Some(changed_at) = locked.password_changed_at() {
            wait_past_cutoff(changed_at).await;
        }
        let is_verified = locked.is_verified();
        let requires_verification = !is_verified;

        let tokens = self.token_issuer.issue(&user_id, is_verified).await?;
        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user_id,
            tokens.refresh_token_expires_at,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
        );
        events.push(IdentityEvent::from(UserEvent::LoggedIn(UserLoggedIn::record(
            user_id,
            tokens.refresh_token_id,
            request.client.ip,
            request.client.user_agent,
            Utc::now(),
        ))));
        tx.insert_refresh_token(&refresh_token).await?;
        tx.save_events(&events, request.ctx.request_id.as_deref())
            .await?;
        tx.commit().await?;

        Ok(LoginOutcome {
            tokens,
            requires_verification,
        })
    }
}
