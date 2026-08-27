use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::repository::UserRepository, verification::channel::VerificationChannel, verification::code::VerificationCode, verification::repository::VerificationCodeRepository};

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

pub struct ConfirmRegistration<T, H, V, W>
where
    T: UserRepository,
    H: Hasher,
    V: VerificationCodeRepository,
    W: IdentityUnitOfWork,
{
    user_repository: Arc<T>,
    hasher: Arc<H>,
    verification_code_repository: Arc<V>,
    unit_of_work: Arc<W>,
}

impl<T, H, V, W> ConfirmRegistration<T, H, V, W>
where
    T: UserRepository,
    H: Hasher,
    V: VerificationCodeRepository,
    W: IdentityUnitOfWork,
{
    pub fn new(
        user_repository: Arc<T>,
        hasher: Arc<H>,
        verification_code_repository: Arc<V>,
        unit_of_work: Arc<W>,
    ) -> Self {
        Self {
            user_repository,
            hasher,
            verification_code_repository,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        request: ConfirmRegistrationRequest,
    ) -> Result<(), ConfirmRegistrationError> {
        let mut user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(ConfirmRegistrationError::UserNotFound)?;

        if user.is_verified() {
            return Err(ConfirmRegistrationError::UserAlreadyVerified);
        }

        let mut verification_code = self
            .verification_code_repository
            .find_verification_code_by_user_id(&user_id)
            .await?;

        if !verification_code.is_registration_purpose() {
            return Err(ConfirmRegistrationError::NotRegistration);
        }

        if verification_code.is_limit_attempts() {
            return Err(ConfirmRegistrationError::TooManyAttempts);
        }

        let now = Utc::now();

        if verification_code.expires_at() < now {
            return Err(ConfirmRegistrationError::VerificationCodeExpired);
        }

        if !self.hasher.verify(
            request.verification_code.as_str(),
            verification_code.code_hash().as_str(),
        )? {
            let incremented = self
                .verification_code_repository
                .increment_attempts(verification_code.id(), VerificationCode::max_attempts())
                .await?;

            if incremented.is_none() {
                return Err(ConfirmRegistrationError::TooManyAttempts);
            }

            return Err(ConfirmRegistrationError::BadVerificationCode);
        }

        match verification_code.channel() {
            VerificationChannel::Email => user.verify_email(now),
            VerificationChannel::Phone => user.verify_phone(now),
        }

        verification_code.set_consumed_at(Some(now));

        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();

        let mut tx = self.unit_of_work.begin().await?;
        tx.mark_user_verified(&user).await?;
        if !tx.consume_verification_code(&verification_code).await? {
            return Err(ConfirmRegistrationError::UserAlreadyVerified);
        }
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(())
    }
}
