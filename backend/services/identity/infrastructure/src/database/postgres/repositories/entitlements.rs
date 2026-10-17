use async_trait::async_trait;
use domain::user::id::UserId;
use identity_domain::user::tier::{Entitlements, EntitlementsRepository, Tier};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresEntitlementsRepository {
    pool: Pool<Postgres>,
}

impl PostgresEntitlementsRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub(crate) async fn load_entitlements<'e, E>(executor: E, user_id: &UserId) -> Result<Entitlements, RepositoryError>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let row = sqlx::query!(
        r#"SELECT tier, valid_until, member_number, source, trial_used_at FROM user_entitlements WHERE user_id = $1"#,
        user_id.uuid(),
    )
    .fetch_optional(executor)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    Ok(row
        .map(|r| Entitlements {
            tier: Tier::parse(&r.tier).unwrap_or_default(),
            valid_until: r.valid_until,
            member_number: r.member_number,
            trial: r.source == "trial",
            trial_used_at: r.trial_used_at,
        })
        .unwrap_or_default())
}

#[async_trait]
impl EntitlementsRepository for PostgresEntitlementsRepository {
    async fn get(&self, user_id: &UserId) -> Result<Entitlements, RepositoryError> {
        load_entitlements(&self.pool, user_id).await
    }
}
