use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use identity_domain::{user::locale::Locale, user::repository::UserRepository};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum GetMeError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("User not found")]
    NotFound,
}

#[derive(Debug)]
pub struct MeProfile {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub locale: Locale,
}

pub struct GetMe<U>
where
    U: UserRepository,
{
    user_repository: Arc<U>,
}

impl<U> GetMe<U>
where
    U: UserRepository,
{
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<MeProfile, GetMeError> {
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(GetMeError::NotFound)?;

        Ok(MeProfile {
            id: user.id().uuid(),
            name: user.name().as_str().to_string(),
            email: user.email().as_str().to_string(),
            locale: user.locale(),
        })
    }
}
