use std::sync::Arc;

use domain::{
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::{id::UserId, repository::UserRepository},
    verification::{
        channel::VerificationChannel, code::VerificationCode, code_hash::CodeHash,
        purpose::VerificationPurpose, repository::VerificationCodeRepository,
    },
};

use crate::ports::hasher::{HashError, Hasher};

#[derive(Debug, thiserror::Error)]
pub enum IssueVerificationCodeError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Unsupported verification channel")]
    UnsupportedChannel,
    #[error("User already verified")]
    UserAlreadyVerified,
}

pub struct IssueVerificationCode<U, V, H>
where
    U: UserRepository,
    V: VerificationCodeRepository,
    H: Hasher,
{
    user_repository: Arc<U>,
    verification_code_repository: Arc<V>,
    hasher: Arc<H>,
}

impl<U, V, H> IssueVerificationCode<U, V, H>
where
    U: UserRepository,
    V: VerificationCodeRepository,
    H: Hasher,
{
    pub fn new(
        user_repository: Arc<U>,
        verification_code_repository: Arc<V>,
        hasher: Arc<H>,
    ) -> Self {
        Self {
            user_repository,
            verification_code_repository,
            hasher,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        purpose: VerificationPurpose,
        channel: VerificationChannel,
    ) -> Result<(), IssueVerificationCodeError> {
        if !matches!(channel, VerificationChannel::Email) {
            return Err(IssueVerificationCodeError::UnsupportedChannel);
        }

        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(RepositoryError::NotFound)?;

        if matches!(purpose, VerificationPurpose::Registration) && user.is_verified() {
            return Err(IssueVerificationCodeError::UserAlreadyVerified);
        }

        let plain_code = VerificationCode::generate();
        let code_hash = CodeHash::new(self.hasher.hash(&plain_code)?);

        let mut verification_code = VerificationCode::issue(
            user_id,
            user.email().clone(),
            plain_code,
            channel,
            code_hash,
            purpose,
        );

        let records: Vec<OutboxRecord> = verification_code
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record())
            .collect::<Result<Vec<_>, _>>()?;

        self.verification_code_repository
            .save_verification_code_with_events(&verification_code, &records)
            .await?;

        Ok(())
    }
}
