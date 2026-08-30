use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError};
use ports::repository::RepositoryError;

use crate::{
    dto::requests::auth::confirm_registration::ConfirmRegistrationRequest,
    ports::hasher::{HashError, Hasher},
};

#[derive(Debug, thiserror::Error)]
pub enum ConfirmRegistrationError {
    #[error("User already verified")]
    UserAlreadyVerified,
    #[error("Too many attempts for verification code")]
    TooManyAttempts,
    #[error("Verification code has been expired")]
    VerificationCodeExpired,
    #[error("Bad verification code")]
    BadVerificationCode,
    #[error("Verification code not for registration")]
    NotRegistration,
    #[error("User not found")]
    UserNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
}

impl From<UserError> for ConfirmRegistrationError {
    fn from(err: UserError) -> Self {
        match err {
            UserError::AlreadyVerified | UserError::CodeConsumed => Self::UserAlreadyVerified,
            UserError::TooManyAttempts => Self::TooManyAttempts,
            UserError::CodeExpired => Self::VerificationCodeExpired,
            UserError::BadCode => Self::BadVerificationCode,
            UserError::NotRegistrationCode
            | UserError::ForeignCode
            | UserError::NotEmailChangeCode
            | UserError::NoPendingEmail
            | UserError::Deleted => Self::NotRegistration,
        }
    }
}

pub struct ConfirmRegistration<H, W>
where
    H: Hasher,
    W: IdentityUnitOfWork,
{
    hasher: Arc<H>,
    unit_of_work: Arc<W>,
}

impl<H, W> ConfirmRegistration<H, W>
where
    H: Hasher,
    W: IdentityUnitOfWork,
{
    pub fn new(hasher: Arc<H>, unit_of_work: Arc<W>) -> Self {
        Self {
            hasher,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        request: ConfirmRegistrationRequest,
    ) -> Result<(), ConfirmRegistrationError> {
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(ConfirmRegistrationError::UserNotFound)?;
        if user.is_verified() {
            return Err(ConfirmRegistrationError::UserAlreadyVerified);
        }
        let mut code = tx
            .load_pending_verification_for_update(&user_id)
            .await?
            .ok_or(RepositoryError::NotFound)?;

        let now = Utc::now();
        user.ensure_can_confirm(&code, now)?;
        let code_matches = self
            .hasher
            .verify(request.verification_code.as_str(), code.code_hash().as_str())?;
        let outcome = user.confirm_registration(&mut code, code_matches, now);
        tx.save_verification_code(&code).await?;
        match outcome {
            Ok(()) => {
                let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
                tx.save_user(&user).await?;
                tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
                tx.commit().await?;
                Ok(())
            }
            Err(err @ (UserError::BadCode | UserError::TooManyAttempts)) => {
                tx.commit().await?;
                Err(err.into())
            }
            Err(err) => Err(err.into()),
        }
    }
}
