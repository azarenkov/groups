use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;

pub struct GetLinkPreview<U>
where
    U: UserRepository,
{
    user_repository: Arc<U>,
}

impl<U> GetLinkPreview<U>
where
    U: UserRepository,
{
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<Option<String>, RepositoryError> {
        let Some(user) = self.user_repository.find_by_id(&user_id).await? else {
            return Ok(None);
        };
        if user.is_deleted() || user.is_disabled() || !user.is_verified() {
            return Ok(None);
        }
        Ok(Some(user.name().as_str().to_string()))
    }
}

#[cfg(test)]
#[path = "link_preview_tests.rs"]
mod tests;
