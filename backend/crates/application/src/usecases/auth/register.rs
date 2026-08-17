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
    user::{
        User,
        email::Email,
        id::UserId,
        name::{ReservedNames, UserName, UserNameError},
        repository::UserRepository,
    },
};


use crate::{
    dto::requests::auth::register_user::RegisterUserRequest,
    ports::{
        hasher::{HashError, Hasher},
        text_moderation::{TextModeration, TextModerationError},
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
    #[error("Invalid username: {0}")]
    InvalidName(#[from] UserNameError),
    #[error("Username is already taken")]
    NameTaken,
    #[error("Username is not allowed")]
    NameNotAllowed,
}

pub struct RegisteredUser {
    pub user_id: UserId,
    pub email: Email,
    pub tokens: IssuedTokens,
}

pub struct RegisterUser<T, H, I, TM>
where
    T: UserRepository,
    H: Hasher,
    I: TokenIssuer,
    TM: TextModeration,
{
    user_repository: Arc<T>,
    hasher: Arc<H>,
    token_issuer: Arc<I>,
    text_moderation: Arc<TM>,
    reserved_names: Arc<ReservedNames>,
}

impl<T, H, I, TM> RegisterUser<T, H, I, TM>
where
    T: UserRepository,
    H: Hasher,
    I: TokenIssuer,
    TM: TextModeration,
{
    pub fn new(
        user_repository: Arc<T>,
        hasher: Arc<H>,
        token_issuer: Arc<I>,
        text_moderation: Arc<TM>,
        reserved_names: Arc<ReservedNames>,
    ) -> Self {
        Self {
            user_repository,
            hasher,
            token_issuer,
            text_moderation,
            reserved_names,
        }
    }

    pub async fn execute(
        &self,
        request: RegisterUserRequest,
    ) -> Result<RegisteredUser, RegistrationError> {
        let name = UserName::parse(&request.name, &self.reserved_names)?;

        if self.user_repository.find_by_name(&name).await?.is_some() {
            return Err(RegistrationError::NameTaken);
        }

        match self.text_moderation.ensure_allowed(name.as_str()).await {
            Ok(()) => {}
            Err(TextModerationError::Rejected) => {
                return Err(RegistrationError::NameNotAllowed);
            }
            Err(TextModerationError::Unavailable(e)) => {
                tracing::warn!(error = %e, "username moderation unavailable; allowing name");
            }
        }

        let password_hash = PasswordHash::new(self.hasher.hash(&request.password)?);

        let mut user = User::create(
            name,
            Email::new(request.email),
            password_hash,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
            request.locale,
        );

        let tokens = self.token_issuer.issue(user.id(), false).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user.id_ownned(),
            tokens.refresh_token_expires_at,
            request.client.ip,
            request.client.user_agent,
        );

        let request_id = request.ctx.request_id.clone();
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
