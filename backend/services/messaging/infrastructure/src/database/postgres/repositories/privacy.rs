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

    async fn apply_patch(
        &self,
        user_id: &UserId,
        dm_policy: Option<DmPolicy>,
        accept_friend_requests: Option<bool>,
    ) -> Result<PrivacySettings, RepositoryError> {
        let row = sqlx::query!(
            r#"
            INSERT INTO user_privacy_settings (user_id, dm_policy, accept_friend_requests, updated_at)
            VALUES ($1, COALESCE($2, 'friends'), COALESCE($3, true), now())
            ON CONFLICT (user_id) DO UPDATE
                SET dm_policy = COALESCE($2, user_privacy_settings.dm_policy),
                    accept_friend_requests = COALESCE($3, user_privacy_settings.accept_friend_requests),
                    updated_at = now()
            RETURNING dm_policy, accept_friend_requests
            "#,
            user_id.uuid(),
            dm_policy.map(|p| p.as_str().to_string()) as Option<String>,
            accept_friend_requests,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(PrivacySettings {
            dm_policy: DmPolicy::parse(&row.dm_policy).unwrap_or(DmPolicy::Friends),
            accept_friend_requests: row.accept_friend_requests,
        })
    }
}
