use async_trait::async_trait;

use crate::{shared::repository::error::RepositoryError, verification::code::VerificationCode};

#[async_trait]
pub trait VerificationCodeRepository {
    async fn save_verification_code(
        &self,
        verification_code: &VerificationCode,
    ) -> Result<(), RepositoryError>;
}
