use std::sync::Arc;

use domain::{
    shared::{repository::error::RepositoryError, value_objects::password_hash::PasswordHash},
    user::{
        User,
        repository::UserRepository,
        email::Email, name::UserName,
    },
    verification::{
        channel::VerificationChannel, code::VerificationCode, code_hash::CodeHash,
        purpose::VerificationPurpose, repository::VerificationCodeRepository,
    },
};

use crate::{
    dto::requests::auth::register_user::RegisterUserRequest,
    ports::{
        hasher::{HashError, Hasher},
        mailer::{Mailer, MailerError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Mailer error: {0}")]
    Mailer(#[from] MailerError),
}

pub struct RegisterUser<T, H, M, V>
where
    T: UserRepository,
    H: Hasher,
    M: Mailer,
    V: VerificationCodeRepository,
{
    user_repository: Arc<T>,
    hasher: Arc<H>,
    mailer: Arc<M>,
    verification_code_repository: Arc<V>,
}

impl<T, H, M, V> RegisterUser<T, H, M, V>
where
    T: UserRepository,
    H: Hasher,
    M: Mailer,
    V: VerificationCodeRepository,
{
    pub fn new(
        user_repository: Arc<T>,
        hasher: Arc<H>,
        mailer: Arc<M>,
        verification_code_repository: Arc<V>,
    ) -> Self {
        Self {
            user_repository,
            hasher,
            mailer,
            verification_code_repository,
        }
    }

    pub async fn execute(&self, request: RegisterUserRequest) -> Result<User, RegistrationError> {
        let password_hash = PasswordHash::new(self.hasher.hash(&request.password)?);

        let user = User::create(
            UserName::new(request.name),
            Email::new(request.email),
            password_hash,
        );
        self.user_repository.save_user(&user).await?;

        let verification_code = VerificationCode::generate();

        self.mailer
            .send_verification_code(user.email(), &verification_code)
            .await?;

        let verification_code_hash = CodeHash::new(self.hasher.hash(&verification_code)?);

        let verification_code = VerificationCode::create(
            user.id_ownned(),
            VerificationChannel::Email,
            verification_code_hash,
            VerificationPurpose::Registration,
        );

        self.verification_code_repository
            .save_verification_code(&verification_code)
            .await?;

        Ok(user)
    }
}
