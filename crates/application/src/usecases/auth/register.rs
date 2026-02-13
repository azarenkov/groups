use std::sync::Arc;

use domain::{
    entites::user::User,
    repositories::{error::RepositoryError, user::UserRepository},
    value_objects::{
        password_hash::PasswordHash,
        user::{email::Email, name::UserName},
    },
};

use crate::{
    dto::requests::auth::register_user::RegisterUserRequest,
    ports::hasher::{HashError, Hasher},
};

#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
}

pub struct RegisterUser<T, H>
where
    T: UserRepository,
    H: Hasher,
{
    user_repository: Arc<T>,
    hasher: Arc<H>,
}

impl<T, H> RegisterUser<T, H>
where
    T: UserRepository,
    H: Hasher,
{
    pub fn new(user_repository: Arc<T>, hasher: Arc<H>) -> Self {
        Self {
            user_repository,
            hasher,
        }
    }

    pub async fn execute(&self, request: RegisterUserRequest) -> Result<User, RegistrationError> {
        let password_hash = PasswordHash::new(self.hasher.hash(&request.password)?);
        let user = User::create(
            UserName::new(request.name),
            Email::new(request.email),
            password_hash,
        );
        self.user_repository.create_user(&user).await?;
        Ok(user)
    }
}
