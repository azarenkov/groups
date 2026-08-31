use async_trait::async_trait;
use messaging_domain::privacy::{DmPolicy, PrivacyRepository, PrivacySettings};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresPrivacyRepository {
    pool: Pool<Postgres>,
}

impl PostgresPrivacyRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PrivacyRepository for PostgresPrivacyRepository {
    async fn get(&self, user_id: &UserId) -> Result<PrivacySettings, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT dm_policy, accept_friend_requests FROM user_privacy_settings WHERE user_id = $1"#,
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row
            .map(|r| PrivacySettings {
                dm_policy: DmPolicy::parse(&r.dm_policy).unwrap_or(DmPolicy::Friends),
                accept_friend_requests: r.accept_friend_requests,
            })
            .unwrap_or_default())
    }

    async fn set(&self, user_id: &UserId, settings: PrivacySettings) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO user_privacy_settings (user_id, dm_policy, accept_friend_requests, updated_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (user_id) DO UPDATE
                SET dm_policy = EXCLUDED.dm_policy,
                    accept_friend_requests = EXCLUDED.accept_friend_requests,
                    updated_at = now()
            "#,
            user_id.uuid(),
            settings.dm_policy.as_str(),
            settings.accept_friend_requests,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
