use std::sync::Arc;

use domain::{
    refresh_token::{
        refresh_token::RefreshToken, repository::RefreshTokenRepository,
    },
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::{User, email::Email, name::UserName, repository::UserRepository},
    verification::{
        channel::VerificationChannel, code::VerificationCode, code_hash::CodeHash,
        purpose::VerificationPurpose,
    },
};

use crate::{
    dto::requests::auth::register_user::RegisterUserRequest,
    ports::{
        hasher::{HashError, Hasher},
        token_issuer::{IssuedTokens, TokenIssuer, TokenIssuerError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
}

pub struct RegisterUser<T, H, R, I>
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

impl<T, H, R, I> RegisterUser<T, H, R, I>
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

    pub async fn execute(
        &self,
        request: RegisterUserRequest,
    ) -> Result<IssuedTokens, RegistrationError> {
        let password_hash = PasswordHash::new(self.hasher.hash(&request.password)?);

        let user = User::create(
            UserName::new(request.name),
            Email::new(request.email),
            password_hash,
        );

        let plain_code = VerificationCode::generate();
        let code_hash = CodeHash::new(self.hasher.hash(&plain_code)?);

        let mut verification_code = VerificationCode::issue(
            user.id_ownned(),
            user.email().clone(),
            plain_code,
            VerificationChannel::Email,
            code_hash,
            VerificationPurpose::Registration,
        );

        let records: Vec<OutboxRecord> = verification_code
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record())
            .collect::<Result<_, _>>()?;

        self.user_repository
            .register_user(&user, &verification_code, &records)
            .await?;

        let tokens = self.token_issuer.issue(user.id()).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user.id_ownned(),
            tokens.refresh_token_expires_at,
        );
        if let Err(e) = self
            .refresh_token_repository
            .save_refresh_token(&refresh_token)
            .await
        {
            tracing::warn!(
                user_id = %user.id().uuid(),
                error = ?e,
                "Failed to persist refresh token at registration; user will need to re-login when access token expires",
            );
        }

        Ok(tokens)
    }
}
