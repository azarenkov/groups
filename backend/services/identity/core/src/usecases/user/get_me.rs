use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use chrono::Utc;
use identity_domain::{user::emoji_status::EmojiStatus, user::locale::Locale, user::repository::UserRepository};
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
    pub email_verified: bool,
    pub password_changed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub bio: String,
    pub show_activity: bool,
    pub timezone: String,
    pub emoji_status: Option<EmojiStatus>,
    pub profile_updated_at: chrono::DateTime<chrono::Utc>,
    pub appear_offline: bool,
    pub role: domain::user::role::UserRole,
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
        let appear_offline = !self.user_repository.appear_offline_among(std::slice::from_ref(&user_id)).await?.is_empty();

        Ok(MeProfile {
            id: user.id().uuid(),
            name: user.name().as_str().to_string(),
            email: user.email().as_str().to_string(),
            locale: user.locale(),
            email_verified: user.is_email_verified(),
            password_changed_at: user.password_changed_at(),
            bio: user.bio().to_string(),
            show_activity: user.show_activity(),
            timezone: user.timezone().to_string(),
            emoji_status: user.active_emoji_status(Utc::now()),
            profile_updated_at: user.updated_at(),
            appear_offline,
            role: user.role(),
        })
    }
}
