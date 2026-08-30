use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError, verification::purpose::VerificationPurpose};
use ports::repository::RepositoryError;

use crate::ports::hasher::{HashError, Hasher};

#[derive(Debug, thiserror::Error)]
pub enum ConfirmEmailChangeError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("No pending email change")]
    NoPendingChange,
    #[error("Email is already in use")]
    EmailTaken,
    #[error("Too many attempts for verification code")]
    TooManyAttempts,
    #[error("Verification code has expired")]
    CodeExpired,
    #[error("Bad verification code")]
    BadCode,
    #[error("User not found")]
    UserNotFound,
}

impl From<UserError> for ConfirmEmailChangeError {
    fn from(err: UserError) -> Self {
        match err {
            UserError::TooManyAttempts => Self::TooManyAttempts,
            UserError::CodeExpired => Self::CodeExpired,
            UserError::BadCode => Self::BadCode,
            UserError::Deleted => Self::UserNotFound,
            UserError::AlreadyVerified
            | UserError::NotRegistrationCode
            | UserError::CodeConsumed
            | UserError::ForeignCode
            | UserError::NotEmailChangeCode
            | UserError::BioTooLong
            | UserError::NoPendingEmail => Self::NoPendingChange,
        }
    }
}

pub struct ConfirmEmailChangeRequest {
    pub verification_code: String,
    pub request_id: Option<String>,
}

pub struct ConfirmEmailChange<W, H>
where
    W: IdentityUnitOfWork,
    H: Hasher,
{
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
}

impl<W, H> ConfirmEmailChange<W, H>
where
    W: IdentityUnitOfWork,
    H: Hasher,
{
    pub fn new(unit_of_work: Arc<W>, hasher: Arc<H>) -> Self {
        Self { unit_of_work, hasher }
    }

    pub async fn execute(&self, user_id: UserId, request: ConfirmEmailChangeRequest) -> Result<String, ConfirmEmailChangeError> {
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(ConfirmEmailChangeError::UserNotFound)?;
        let mut code = tx
            .load_pending_verification_for_purpose_for_update(&user_id, VerificationPurpose::EmailChange)
            .await?
            .ok_or(ConfirmEmailChangeError::NoPendingChange)?;

        let now = Utc::now();
        user.ensure_can_confirm_email_change(&code, now)?;
        let code_matches = self
            .hasher
            .verify(request.verification_code.trim(), code.code_hash().as_str())?;
        let outcome = user.confirm_email_change(&mut code, code_matches, now);
        tx.save_verification_code(&code).await?;
        match outcome {
            Ok(()) => {
                let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
                match tx.save_user(&user).await {
                    Ok(()) => {}
                    Err(RepositoryError::AlreadyExists) => return Err(ConfirmEmailChangeError::EmailTaken),
                    Err(e) => return Err(e.into()),
                }
                tx.save_events(&events, request.request_id.as_deref()).await?;
                tx.commit().await?;
                Ok(user.email().as_str().to_string())
            }
            Err(err @ (UserError::BadCode | UserError::TooManyAttempts)) => {
                tx.commit().await?;
                Err(err.into())
            }
            Err(err) => Err(err.into()),
        }
    }
}
