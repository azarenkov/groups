use async_trait::async_trait;
use domain::{
    notification::{
        NotificationLevel, NotificationScope,
        repository::{NotificationPreference, NotificationPreferenceRepository},
    },
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresNotificationPreferenceRepository {
    pool: Pool<Postgres>,
}

impl PostgresNotificationPreferenceRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationPreferenceRepository for PostgresNotificationPreferenceRepository {
    async fn list_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<NotificationPreference>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT scope_type, scope_id, level
            FROM notification_preferences
            WHERE user_id = $1
            "#,
            user_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                Some(NotificationPreference {
                    scope: NotificationScope::parse(&row.scope_type)?,
                    scope_id: row.scope_id,
                    level: NotificationLevel::parse(&row.level)?,
                })
            })
            .collect())
    }

    async fn set_preference(
        &self,
        user_id: &UserId,
        preference: &NotificationPreference,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO notification_preferences (user_id, scope_type, scope_id, level, updated_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (user_id, scope_type, scope_id)
            DO UPDATE SET level = EXCLUDED.level, updated_at = EXCLUDED.updated_at
            "#,
            user_id.uuid(),
            preference.scope.as_str(),
            preference.scope_id,
            preference.level.as_str(),
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
