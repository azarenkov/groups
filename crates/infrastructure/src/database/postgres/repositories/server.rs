use async_trait::async_trait;

use domain::{
    server::{Server, repository::ServerRepository, value_objects::id::ServerId},
    shared::repository::error::RepositoryError,
    user::value_objects::id::UserId,
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

    async fn create_server(&self, server: &Server) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                   INSERT INTO servers (id, name, owner_id, description, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6)
               "#,
            server.id().uuid(),
            server.name().as_str(),
            server.owner_id().uuid(),
            server.description().map(|d| d.as_str()),
            server.created_at(),
            server.updated_at()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn delete_server_by_owner(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                DELETE FROM servers WHERE id = $1 AND owner_id = $2
            "#,
            server_id.uuid(),
            owner_id.uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.rows_affected() > 0)
    }
}
