use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError, user::email::Email, user::repository::UserRepository, verification::code::VerificationCode, verification::code_hash::CodeHash};
use ports::repository::RepositoryError;

use crate::ports::hasher::{HashError, Hasher};

#[derive(Debug, thiserror::Error)]
pub enum RequestEmailChangeError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Password is incorrect")]
    InvalidPassword,
    #[error("Email is already in use")]
    EmailTaken,
    #[error("New email matches the current one")]
    SameEmail,
    #[error("User not found")]
    UserNotFound,
}

pub struct RequestEmailChangeRequest {
    pub password: String,
    pub new_email: String,
    pub request_id: Option<String>,
}

pub struct RequestEmailChange<U, W, H>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
}

impl<U, W, H> RequestEmailChange<U, W, H>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
{
    pub fn new(user_repository: Arc<U>, unit_of_work: Arc<W>, hasher: Arc<H>) -> Self {
        Self {
            user_repository,
            unit_of_work,
            hasher,
        }
    }

    pub async fn execute(&self, user_id: UserId, request: RequestEmailChangeRequest) -> Result<(), RequestEmailChangeError> {
        let new_email = Email::new(request.new_email.trim().to_lowercase());
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(RequestEmailChangeError::UserNotFound)?;
        if user.is_deleted() {
            return Err(RequestEmailChangeError::UserNotFound);
        }
        if !self
            .hasher
            .verify(&request.password, user.password_hash().as_str())?
        {
            return Err(RequestEmailChangeError::InvalidPassword);
        }
        if user.email().as_str().eq_ignore_ascii_case(new_email.as_str()) {
            return Err(RequestEmailChangeError::SameEmail);
        }
        if self.user_repository.find_by_email(&new_email).await?.is_some() {
            return Err(RequestEmailChangeError::EmailTaken);
        }

        let plain_code = VerificationCode::generate();
        let code_hash = CodeHash::new(self.hasher.hash(&plain_code)?);

        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(RequestEmailChangeError::UserNotFound)?;
        let mut code = user
            .request_email_change(new_email, plain_code, code_hash, Utc::now())
            .map_err(|e| match e {
                UserError::Deleted => RequestEmailChangeError::UserNotFound,
                _ => RequestEmailChangeError::UserNotFound,
            })?;
        let mut events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        events.extend(code.pull_events().into_iter().map(IdentityEvent::from));
        tx.save_user(&user).await?;
        tx.insert_verification_code(&code).await?;
        tx.save_events(&events, request.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
