use std::sync::Arc;

use domain::{
    notification::repository::{NotificationPreference, NotificationPreferenceRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum ListNotificationPreferencesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListNotificationPreferences<R: NotificationPreferenceRepository> {
    repo: Arc<R>,
}

impl<R: NotificationPreferenceRepository> ListNotificationPreferences<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Vec<NotificationPreference>, ListNotificationPreferencesError> {
        Ok(self.repo.list_for_user(&user_id).await?)
    }
}
