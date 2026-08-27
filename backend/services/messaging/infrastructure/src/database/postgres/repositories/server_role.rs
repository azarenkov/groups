use async_trait::async_trait;
use messaging_domain::{server::id::ServerId, server::role::ServerRole, server::role::ServerRoleRepository};
use domain::{shared::repository::error::RepositoryError, user::id::UserId};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresServerRoleRepository {
    pool: Pool<Postgres>,
}

impl PostgresServerRoleRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServerRoleRepository for PostgresServerRoleRepository {
    async fn list_roles(&self, server_id: &ServerId) -> Result<Vec<ServerRole>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, server_id, name, color, permissions, position, created_at
            FROM server_roles
            WHERE server_id = $1
            ORDER BY position DESC, created_at ASC
            "#,
            server_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rows
            .into_iter()
            .map(|r| ServerRole {
                id: r.id,
                server_id: r.server_id,
                name: r.name,
                color: r.color,
                permissions: r.permissions,
                position: r.position,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn member_role_ids(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<(Uuid, Uuid)>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT mr.user_id, mr.role_id
            FROM server_member_roles mr
            INNER JOIN server_members m
                ON m.server_id = mr.server_id AND m.user_id = mr.user_id
            WHERE mr.server_id = $1
            "#,
            server_id.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|r| (r.user_id, r.role_id)).collect())
    }

    async fn user_permissions(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<i64, RepositoryError> {
        let perms: Option<i64> = sqlx::query_scalar!(
            r#"
            SELECT bit_or(r.permissions) AS "perms"
            FROM server_member_roles mr
            INNER JOIN server_roles r ON r.id = mr.role_id
            INNER JOIN server_members m
                ON m.server_id = mr.server_id AND m.user_id = mr.user_id
            WHERE mr.server_id = $1 AND mr.user_id = $2
            "#,
            server_id.uuid(),
            user_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(perms.unwrap_or(0))
    }
}
