use std::sync::Arc;

use chrono::Utc;
use domain::{password_hash::PasswordHash, user::email::Email};
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError, user::repository::UserRepository, verification::code::VerificationCode, verification::id::VerificationCodeId, verification::code_hash::CodeHash, verification::purpose::VerificationPurpose, verification::throttle::IssuedCodes};
use ports::repository::RepositoryError;

use crate::ports::{
    access_revoker::{AccessRevoker, AccessRevokerError},
    hasher::{HashError, Hasher},
};

#[derive(Debug, thiserror::Error)]
pub enum RequestPasswordResetError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
}

pub struct RequestPasswordResetRequest {
    pub email: String,
    pub request_id: Option<String>,
}

pub struct RequestPasswordReset<U, W, H>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
}

impl<U, W, H> RequestPasswordReset<U, W, H>
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

    pub async fn execute(&self, request: RequestPasswordResetRequest) -> Result<(), RequestPasswordResetError> {
        let typed = request.email.trim();
        let secret = VerificationCode::generate_secret();
        let code_hash = CodeHash::new(self.hasher.hash(&secret)?);
        let mut found = self.user_repository.find_by_email(&Email::new(typed.to_string())).await?;
        if found.is_none() && typed != typed.to_lowercase() {
            found = self.user_repository.find_by_email(&Email::new(typed.to_lowercase())).await?;
        }
        let Some(user) = found else {
            return Ok(());
        };
        if user.is_deleted() {
            return Ok(());
        }
        let user_id = *user.id();

        let mut tx = self.unit_of_work.begin().await?;
        let Some(user) = tx.load_user_for_update(&user_id).await? else {
            return Ok(());
        };
        let now = Utc::now();
        let issued = tx
            .issued_verification_codes(&user_id, VerificationPurpose::PasswordReset, IssuedCodes::window_start(now))
            .await?;
        if issued.retry_after(now).is_some() {
            tracing::info!(user_id = %user_id.uuid(), "Password reset requested too often; skipping");
            return Ok(());
        }
        let code_id = VerificationCodeId::generate();
        let token = reset_token(&code_id, &secret);
        let mut code = match user.issue_password_reset(code_id, token, code_hash) {
            Ok(code) => code,
            Err(_) => return Ok(()),
        };
        let events: Vec<IdentityEvent> = code.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.consume_pending_verifications(&user_id, VerificationPurpose::PasswordReset, now).await?;
        tx.insert_verification_code(&code).await?;
        tx.save_events(&events, request.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResetPasswordError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Access revocation error: {0}")]
    AccessRevoker(#[from] AccessRevokerError),
    #[error("Reset link is invalid or already used")]
    InvalidToken,
    #[error("Reset link has expired")]
    Expired,
}

pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
    pub request_id: Option<String>,
}

pub struct ResetPassword<W, H, R>
where
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
{
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
    access_revoker: Arc<R>,
}

impl<W, H, R> ResetPassword<W, H, R>
where
    W: IdentityUnitOfWork,
    H: Hasher,
    R: AccessRevoker,
{
    pub fn new(unit_of_work: Arc<W>, hasher: Arc<H>, access_revoker: Arc<R>) -> Self {
        Self {
            unit_of_work,
            hasher,
            access_revoker,
        }
    }

    pub async fn execute(&self, request: ResetPasswordRequest) -> Result<(), ResetPasswordError> {
        let (code_id, secret) = parse_reset_token(&request.token).ok_or(ResetPasswordError::InvalidToken)?;

        let mut tx = self.unit_of_work.begin().await?;
        let user_id = tx
            .verification_code_owner(&code_id)
            .await?
            .ok_or(ResetPasswordError::InvalidToken)?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(ResetPasswordError::InvalidToken)?;
        let mut code = tx
            .load_verification_code_for_update(&code_id)
            .await?
            .ok_or(ResetPasswordError::InvalidToken)?;
        let now = Utc::now();
        user.ensure_can_reset_password(&code, now).map_err(reset_error)?;
        let matches = self.hasher.verify(secret, code.code_hash().as_str())?;
        let new_hash = if matches {
            PasswordHash::new(self.hasher.hash(&request.new_password)?)
        } else {
            PasswordHash::new(String::new())
        };
        match user.reset_password(&mut code, matches, new_hash, now) {
            Ok(()) => {}
            Err(err @ (UserError::BadCode | UserError::TooManyAttempts)) => {
                tx.save_verification_code(&code).await?;
                tx.commit().await?;
                return Err(reset_error(err));
            }
            Err(err) => return Err(reset_error(err)),
        }
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_verification_code(&code).await?;
        tx.save_user(&user).await?;
        tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
        tx.save_events(&events, request.request_id.as_deref()).await?;
        self.access_revoker.revoke_access_issued_before(&user_id, now).await?;
        tx.commit().await?;
        Ok(())
    }
}

fn reset_error(err: UserError) -> ResetPasswordError {
    match err {
        UserError::CodeExpired => ResetPasswordError::Expired,
        _ => ResetPasswordError::InvalidToken,
    }
}

pub fn reset_token(code_id: &VerificationCodeId, secret: &str) -> String {
    format!("{}.{}", code_id.uuid().simple(), secret)
}

fn parse_reset_token(token: &str) -> Option<(VerificationCodeId, &str)> {
    let (id, secret) = token.trim().split_once('.')?;
    if id.len() != 32 || secret.len() != 64 || !secret.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    VerificationCodeId::from_string(id).ok().map(|id| (id, secret))
}

#[cfg(test)]
#[path = "password_reset_tests.rs"]
mod tests;
