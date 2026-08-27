use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use identity_domain::{user::locale::Locale, user::repository::UserRepository};

#[derive(Debug, thiserror::Error)]
pub enum UpdateLocaleError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct UpdateUserLocale<U>
where
    U: UserRepository,
{
    user_repository: Arc<U>,
}

impl<U> UpdateUserLocale<U>
where
    U: UserRepository,
{
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        locale: Locale,
    ) -> Result<(), UpdateLocaleError> {
        self.user_repository.update_locale(&user_id, locale).await?;
        Ok(())
    }
}
