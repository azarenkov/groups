use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, refresh_token::RefreshToken, unit_of_work::IdentityUnitOfWork, user::UserError, verification::purpose::VerificationPurpose};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        access_revoker::{AccessRevoker, AccessRevokerError},
        hasher::{HashError, Hasher},
        token_issuer::{IssuedTokens, TokenIssuer, TokenIssuerError},
    },
    usecases::auth::reactivation::wait_past_cutoff,
};

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
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
    #[error("Access revocation error: {0}")]
    AccessRevoker(#[from] AccessRevokerError),
}

pub struct ConfirmedEmailChange {
    pub email: String,
    pub tokens: IssuedTokens,
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
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

pub struct ConfirmEmailChange<W, H, I, R>
where
    W: IdentityUnitOfWork,
    H: Hasher,
    I: TokenIssuer,
    R: AccessRevoker,
{
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
    token_issuer: Arc<I>,
    access_revoker: Arc<R>,
}

impl<W, H, I, R> ConfirmEmailChange<W, H, I, R>
where
    W: IdentityUnitOfWork,
    H: Hasher,
    I: TokenIssuer,
    R: AccessRevoker,
{
    pub fn new(unit_of_work: Arc<W>, hasher: Arc<H>, token_issuer: Arc<I>, access_revoker: Arc<R>) -> Self {
        Self {
            unit_of_work,
            hasher,
            token_issuer,
            access_revoker,
        }
    }

    pub async fn execute(&self, user_id: UserId, request: ConfirmEmailChangeRequest) -> Result<ConfirmedEmailChange, ConfirmEmailChangeError> {
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
                let cutoff = Utc::now();
                tx.revoke_refresh_tokens_for_user(&user_id, None, cutoff).await?;
                wait_past_cutoff(cutoff).await;
                let tokens = self.token_issuer.issue(&user_id, user.is_verified()).await?;
                let refresh_token = RefreshToken::issue(
                    tokens.refresh_token_id,
                    user_id,
                    tokens.refresh_token_expires_at,
                    request.ip,
                    request.user_agent,
                );
                tx.insert_refresh_token(&refresh_token).await?;
                tx.save_events(&events, request.request_id.as_deref()).await?;
                self.access_revoker.revoke_access_issued_before(&user_id, cutoff).await?;
                tx.commit().await?;
                Ok(ConfirmedEmailChange {
                    email: user.email().as_str().to_string(),
                    tokens,
                })
            }
            Err(err @ (UserError::BadCode | UserError::TooManyAttempts)) => {
                tx.commit().await?;
                Err(err.into())
            }
            Err(err) => Err(err.into()),
        }
    }
}
