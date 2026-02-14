use async_trait::async_trait;
use domain::{
    repositories::{error::RepositoryError, server::ServerRepository},
    value_objects::server::id::ServerId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresServerRepository {
    pool: Pool<Postgres>,
}

impl PostgresServerRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServerRepository for PostgresServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                SELECT EXISTS(SELECT 1 FROM servers WHERE id = $1) as "exists!"
            "#,
            server_id.uuid()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.exists)
    }
}
