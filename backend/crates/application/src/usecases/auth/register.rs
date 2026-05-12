use std::sync::Arc;

use chrono::Utc;
use domain::{
    refresh_token::RefreshToken,
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::{
        User,
        email::Email,
        events::{UserEvent, lifecycle::UserLoggedIn},
        name::UserName,
        repository::UserRepository,
    },
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

pub struct RegisterUser<T, H, I>
where
    T: UserRepository,
    H: Hasher,
    I: TokenIssuer,
{
    user_repository: Arc<T>,
    hasher: Arc<H>,
    token_issuer: Arc<I>,
}

impl<T, H, I> RegisterUser<T, H, I>
where
    T: UserRepository,
    H: Hasher,
    I: TokenIssuer,
{
    pub fn new(user_repository: Arc<T>, hasher: Arc<H>, token_issuer: Arc<I>) -> Self {
        Self {
            user_repository,
            hasher,
            token_issuer,
        }
    }

    pub async fn execute(
        &self,
        request: RegisterUserRequest,
    ) -> Result<IssuedTokens, RegistrationError> {
        let password_hash = PasswordHash::new(self.hasher.hash(&request.password)?);

        let mut user = User::create(
            UserName::new(request.name),
            Email::new(request.email),
            password_hash,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
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

        let tokens = self.token_issuer.issue(user.id()).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user.id_ownned(),
            tokens.refresh_token_expires_at,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
        );

        let login_event = UserEvent::LoggedIn(UserLoggedIn::record(
            user.id_ownned(),
            tokens.refresh_token_id,
            request.client.ip,
            request.client.user_agent,
            Utc::now(),
        ));

        let user_records = user
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record())
            .collect::<Result<Vec<_>, _>>()?;
        let verification_records = verification_code
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record())
            .collect::<Result<Vec<_>, _>>()?;
        let records: Vec<OutboxRecord> = user_records
            .into_iter()
            .chain(verification_records)
            .chain(std::iter::once(login_event.to_outbox_record()?))
            .collect();

        self.user_repository
            .register_user(&user, &verification_code, &refresh_token, &records)
            .await?;

        Ok(tokens)
    }
}
