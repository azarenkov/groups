use domain::repositories::user::UserRepository;

use crate::{dto::requests::user::login_user::LoginUserRequest, ports::hasher::Hasher};

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("Invalid credentials")]
    InvalidCredentials,
}

pub struct LoginUser<T, H>
where
    T: UserRepository,
    H: Hasher,
{
    user_repository: T,
    hasher: H,
}

impl<T, H> LoginUser<T, H>
where
    T: UserRepository,
    H: Hasher,
{
    pub fn new(user_repository: T, hasher: H) -> Self {
        Self {
            user_repository,
            hasher,
        }
    }

    pub async fn execute(&self, request: LoginUserRequest) -> Result<String, LoginError> {
        todo!()
    }
}
