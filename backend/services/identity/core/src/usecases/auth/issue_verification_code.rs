use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::repository::UserRepository, verification::channel::VerificationChannel, verification::code::VerificationCode, verification::code_hash::CodeHash, verification::purpose::VerificationPurpose};

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
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        hasher: Arc<H>,
    ) -> Self {
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

        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(RepositoryError::NotFound)?;

        if matches!(request.purpose, VerificationPurpose::Registration) && user.is_verified() {
            return Err(IssueVerificationCodeError::UserAlreadyVerified);
        }

        let plain_code = VerificationCode::generate();
        let code_hash = CodeHash::new(self.hasher.hash(&plain_code)?);

        let mut verification_code = VerificationCode::issue(
            user_id,
            user.email().clone(),
            plain_code,
            request.channel,
            code_hash,
            request.purpose,
            user.locale(),
        );

        let events: Vec<IdentityEvent> = verification_code
            .pull_events()
            .into_iter()
            .map(IdentityEvent::from)
            .collect();

        let mut tx = self.unit_of_work.begin().await?;
        tx.insert_verification_code(&verification_code).await?;
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(())
    }
}
