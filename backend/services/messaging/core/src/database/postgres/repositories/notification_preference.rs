use async_trait::async_trait;
use domain::{
    notification::{
        NotificationLevel, NotificationScope,
        repository::{
            NotificationPreference, NotificationPreferenceRepository,
            StoredNotificationPreference,
        },
    },
    shared::{
        outbox::outbox_record::{AggregateType, OutboxRecord},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

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
    ) -> Result<Vec<StoredNotificationPreference>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT scope_type, scope_id, level, updated_at
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
                Some(StoredNotificationPreference {
                    preference: NotificationPreference {
                        scope: NotificationScope::parse(&row.scope_type)?,
                        scope_id: row.scope_id,
                        level: NotificationLevel::parse(&row.level)?,
                    },
                    updated_at: row.updated_at,
                })
            })
            .collect())
    }

    async fn set_preference(
        &self,
        user_id: &UserId,
        preference: &NotificationPreference,
    ) -> Result<chrono::DateTime<chrono::Utc>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let updated_at = sqlx::query_scalar!(
            r#"
            INSERT INTO notification_preferences (user_id, scope_type, scope_id, level, updated_at)
            VALUES ($1, $2, $3, $4, now())
            ON CONFLICT (user_id, scope_type, scope_id)
            DO UPDATE SET level = EXCLUDED.level, updated_at = EXCLUDED.updated_at
            RETURNING updated_at
            "#,
            user_id.uuid(),
            preference.scope.as_str(),
            preference.scope_id,
            preference.level.as_str(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let record = OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: user_id.uuid(),
            aggregate_type: AggregateType::User,
            parent_id: None,
            parent_type: None,
            event_type: "notification_preference.updated".to_string(),
            payload: serde_json::json!({
                "user_id": user_id.uuid(),
                "scope_type": preference.scope.as_str(),
                "scope_id": preference.scope_id,
                "level": preference.level.as_str(),
                "updated_at": updated_at,
                "recipients": [user_id.uuid()],
            }),
            occurred_at: updated_at,
            request_id: None,
        };
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(updated_at)
    }
}
