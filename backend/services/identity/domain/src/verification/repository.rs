use async_trait::async_trait;

use ports::repository::RepositoryError;
use crate::{user::id::UserId, verification::code::VerificationCode, verification::id::VerificationCodeId};

#[async_trait]
pub trait VerificationCodeRepository {
    async fn save_verification_code(
        &self,
        verification_code: &VerificationCode,
    ) -> Result<(), RepositoryError>;
    async fn find_verification_code_by_user_id(
        &self,
        user_id: &UserId,
    ) -> Result<VerificationCode, RepositoryError>;
    async fn increment_attempts(
        &self,
        id: &VerificationCodeId,
        max_attempts: u8,
    ) -> Result<Option<u8>, RepositoryError>;
}
