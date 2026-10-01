use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use chrono::Utc;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError, user::repository::UserRepository, verification::channel::VerificationChannel, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::throttle::IssuedCodes};

use crate::{
    dto::requests::auth::issue_verification_code::IssueVerificationCodeRequest,
    ports::hasher::{HashError, Hasher},
};

#[derive(Debug, thiserror::Error)]
pub enum IssueVerificationCodeError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Unsupported verification channel")]
    UnsupportedChannel,
    #[error("User already verified")]
    UserAlreadyVerified,
    #[error("User not found")]
    UserNotFound,
    #[error("Try again in {0} seconds")]
    TooSoon(i64),
}

impl From<UserError> for IssueVerificationCodeError {
    fn from(err: UserError) -> Self {
        match err {
            UserError::Deleted => Self::UserNotFound,
            _ => Self::UserAlreadyVerified,
        }
    }
}

pub struct IssueVerificationCode<U, W, H>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
}

impl<U, W, H> IssueVerificationCode<U, W, H>
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

    pub async fn execute(
        &self,
        user_id: UserId,
        request: IssueVerificationCodeRequest,
    ) -> Result<(), IssueVerificationCodeError> {
        if !matches!(request.channel, VerificationChannel::Email) {
            return Err(IssueVerificationCodeError::UnsupportedChannel);
        }

        self.user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(RepositoryError::NotFound)?
            .ensure_can_request_verification(request.purpose)?;

        let plain_code = VerificationCode::generate();
        let code_hash = CodeHash::new(self.hasher.hash(&plain_code)?);

        let mut tx = self.unit_of_work.begin().await?;
        let user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(RepositoryError::NotFound)?;
        let now = Utc::now();
        let issued = tx
            .issued_verification_codes(&user_id, request.purpose, IssuedCodes::window_start(now))
            .await?;
        if let Some(wait) = issued.retry_after(now) {
            return Err(IssueVerificationCodeError::TooSoon(wait.num_seconds().max(1)));
        }

        let mut verification_code =
            user.issue_verification(request.channel, request.purpose, plain_code, code_hash)?;

        let events: Vec<IdentityEvent> = verification_code
            .pull_events()
            .into_iter()
            .map(IdentityEvent::from)
            .collect();

        tx.insert_verification_code(&verification_code).await?;
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(())
    }
}
