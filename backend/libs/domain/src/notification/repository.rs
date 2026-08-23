use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    notification::{NotificationLevel, NotificationScope},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, Clone)]
pub struct NotificationPreference {
    pub scope: NotificationScope,
    pub scope_id: Uuid,
    pub level: NotificationLevel,
}

#[derive(Debug, Clone)]
pub struct StoredNotificationPreference {
    pub preference: NotificationPreference,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait NotificationPreferenceRepository {
    async fn list_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<StoredNotificationPreference>, RepositoryError>;

    async fn set_preference(
        &self,
        user_id: &UserId,
        preference: &NotificationPreference,
    ) -> Result<DateTime<Utc>, RepositoryError>;
}
