use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError, verification::channel::VerificationChannel, verification::code::VerificationCode, verification::code_hash::CodeHash};

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
}

impl From<UserError> for IssueVerificationCodeError {
    fn from(err: UserError) -> Self {
        match err {
            UserError::Deleted => Self::UserNotFound,
            _ => Self::UserAlreadyVerified,
        }
    }
}

pub struct IssueVerificationCode<W, H>
where
    W: IdentityUnitOfWork,
    H: Hasher,
{
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
}

impl<W, H> IssueVerificationCode<W, H>
where
    W: IdentityUnitOfWork,
    H: Hasher,
{
    pub fn new(unit_of_work: Arc<W>, hasher: Arc<H>) -> Self {
        Self {
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

        let plain_code = VerificationCode::generate();
        let code_hash = CodeHash::new(self.hasher.hash(&plain_code)?);

        let mut tx = self.unit_of_work.begin().await?;
        let user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(RepositoryError::NotFound)?;

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
