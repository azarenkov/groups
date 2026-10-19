use std::sync::Arc;

use identity_domain::{events::IdentityEvent, refresh_token::RefreshToken, unit_of_work::IdentityUnitOfWork, user::User, user::email::Email, user::repository::UserRepository};
use domain::{password_hash::PasswordHash, user::id::UserId, user::name::ReservedNames, user::name::UserName, user::name::UserNameError};
use ports::repository::RepositoryError;


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
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
    #[error("Invalid username: {0}")]
    InvalidName(#[from] UserNameError),
    #[error("Username is already taken")]
    NameTaken,
    #[error("Username is not allowed")]
    NameNotAllowed,
    #[error("Username moderation unavailable: {0}")]
    ModerationUnavailable(String),
}

pub struct RegisteredUser {
    pub user_id: UserId,
    pub email: Email,
    pub tokens: IssuedTokens,
}

pub struct RegisterUser<T, W, H, I, TM>
where
    T: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    I: TokenIssuer,
    TM: TextModeration,
{
    user_repository: Arc<T>,
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
    token_issuer: Arc<I>,
    text_moderation: Arc<TM>,
    reserved_names: Arc<ReservedNames>,
}

impl<T, W, H, I, TM> RegisterUser<T, W, H, I, TM>
where
    T: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    I: TokenIssuer,
    TM: TextModeration,
{
    pub fn new(
        user_repository: Arc<T>,
        unit_of_work: Arc<W>,
        hasher: Arc<H>,
        token_issuer: Arc<I>,
        text_moderation: Arc<TM>,
        reserved_names: Arc<ReservedNames>,
    ) -> Self {
        Self {
            user_repository,
            unit_of_work,
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
                return Err(RegistrationError::ModerationUnavailable(e));
            }
        }

        let password_hash = PasswordHash::new(self.hasher.hash(&request.password)?);

        let name_check = name.clone();
        let mut user = User::create(
            name,
            Email::new(request.email),
            password_hash,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
            request.locale,
            request.role,
        );

        let tokens = self.token_issuer.issue(user.id(), false, user.role()).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user.id_ownned(),
            tokens.refresh_token_expires_at,
            request.client.ip,
            request.client.user_agent,
        );

        let request_id = request.ctx.request_id.clone();
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();

        let user_id = user.id_ownned();
        let email = user.email().clone();

        let persisted = async {
            let mut tx = self.unit_of_work.begin().await?;
            tx.insert_user(&user).await?;
            tx.insert_refresh_token(&refresh_token).await?;
            tx.save_events(&events, request_id.as_deref()).await?;
            tx.commit().await
        }
        .await;
        match persisted {
            Ok(()) => {}
            Err(RepositoryError::AlreadyExists) => {
                if self.user_repository.find_by_name(&name_check).await?.is_some() {
                    return Err(RegistrationError::NameTaken);
                }
                return Err(RegistrationError::Repository(RepositoryError::AlreadyExists));
            }
            Err(e) => return Err(e.into()),
        }

        Ok(RegisteredUser {
            user_id,
            email,
            tokens,
        })
    }
}
