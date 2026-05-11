use std::sync::Arc;

use domain::{
    refresh_token::{
        refresh_token::RefreshToken, repository::RefreshTokenRepository,
    },
    shared::{repository::error::RepositoryError, value_objects::password_hash::PasswordHash},
    user::{User, email::Email, name::UserName, repository::UserRepository},
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
        token_issuer::{IssuedTokens, TokenIssuer, TokenIssuerError},
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
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
}

pub struct RegisterUser<T, H, M, V, R, I>
where
    T: UserRepository,
    H: Hasher,
    M: Mailer,
    V: VerificationCodeRepository,
    R: RefreshTokenRepository,
    I: TokenIssuer,
{
    user_repository: Arc<T>,
    hasher: Arc<H>,
    mailer: Arc<M>,
    verification_code_repository: Arc<V>,
    refresh_token_repository: Arc<R>,
    token_issuer: Arc<I>,
}

impl<T, H, M, V, R, I> RegisterUser<T, H, M, V, R, I>
where
    T: UserRepository,
    H: Hasher,
    M: Mailer,
    V: VerificationCodeRepository,
    R: RefreshTokenRepository,
    I: TokenIssuer,
{
    pub fn new(
        user_repository: Arc<T>,
        hasher: Arc<H>,
        mailer: Arc<M>,
        verification_code_repository: Arc<V>,
        refresh_token_repository: Arc<R>,
        token_issuer: Arc<I>,
    ) -> Self {
        Self {
            user_repository,
            hasher,
            mailer,
            verification_code_repository,
            refresh_token_repository,
            token_issuer,
        }
    }

    pub async fn execute(
        &self,
        request: RegisterUserRequest,
    ) -> Result<IssuedTokens, RegistrationError> {
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

        let tokens = self.token_issuer.issue(user.id()).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user.id_ownned(),
            tokens.refresh_token_expires_at,
        );
        self.refresh_token_repository
            .save_refresh_token(&refresh_token)
            .await?;

        Ok(tokens)
    }
}
