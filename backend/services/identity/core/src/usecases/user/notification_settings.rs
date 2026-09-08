use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::unit_of_work::IdentityUnitOfWork;
use identity_domain::user::repository::UserRepository;
use identity_domain::user::settings::notification::{
    NotificationSettings, NotificationSettingsError as DomainNotificationSettingsError,
    NotificationSettingsPatch,
};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum NotificationSettingsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid notification settings: {0}")]
    Invalid(#[from] DomainNotificationSettingsError),
}

pub struct GetNotificationSettings<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> GetNotificationSettings<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<NotificationSettings>, NotificationSettingsError> {
        Ok(self.user_repository.get_notification_settings(&user_id).await?)
    }
}

pub struct UpdateNotificationSettings<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> UpdateNotificationSettings<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: NotificationSettingsPatch,
    ) -> Result<NotificationSettings, NotificationSettingsError> {
        let mut tx = self.unit_of_work.begin().await?;
        let current = tx
            .load_notification_settings_for_update(&user_id)
            .await?
            .unwrap_or_default();
        let next = current.apply(&patch)?;
        tx.save_notification_settings(&user_id, &next).await?;
        tx.commit().await?;
        Ok(next)
    }
}
