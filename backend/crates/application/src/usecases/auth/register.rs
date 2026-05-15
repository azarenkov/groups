use std::sync::Arc;

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
    user::{User, email::Email, id::UserId, name::UserName, repository::UserRepository},
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

pub struct RegisteredUser {
    pub user_id: UserId,
    pub email: Email,
    pub tokens: IssuedTokens,
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
    ) -> Result<RegisteredUser, RegistrationError> {
        let password_hash = PasswordHash::new(self.hasher.hash(&request.password)?);

        let mut user = User::create(
            UserName::new(request.name),
            Email::new(request.email),
            password_hash,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
        );

        let tokens = self.token_issuer.issue(user.id(), false).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user.id_ownned(),
            tokens.refresh_token_expires_at,
            request.client.ip,
            request.client.user_agent,
        );

        let request_id = shared::middleware::request_id::current();
        let records: Vec<OutboxRecord> = user
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        let user_id = user.id_ownned();
        let email = user.email().clone();

        self.user_repository
            .register_user(&user, &refresh_token, &records)
            .await?;

        Ok(RegisteredUser {
            user_id,
            email,
            tokens,
        })
    }
}
