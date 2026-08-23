use async_trait::async_trait;
use domain::{
    server::{
        id::ServerId,
        role::{ServerRole, ServerRoleRepository},
    },
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

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

    async fn create_role(
        &self,
        role: &ServerRole,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            INSERT INTO server_roles (id, server_id, name, color, permissions, position, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            role.id,
            role.server_id,
            role.name,
            role.color,
            role.permissions,
            role.position,
            role.created_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn update_role(
        &self,
        role: &ServerRole,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
            UPDATE server_roles
            SET name = $3, color = $4, permissions = $5, position = $6
            WHERE id = $1 AND server_id = $2
            "#,
            role.id,
            role.server_id,
            role.name,
            role.color,
            role.permissions,
            role.position,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn delete_role(
        &self,
        server_id: &ServerId,
        role_id: Uuid,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
            DELETE FROM server_roles
            WHERE id = $1 AND server_id = $2
            "#,
            role_id,
            server_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn assign_role(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
        role_id: Uuid,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let valid = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM server_roles r
                INNER JOIN server_members m
                    ON m.server_id = r.server_id AND m.user_id = $3
                WHERE r.id = $1 AND r.server_id = $2
            ) AS "exists!"
            "#,
            role_id,
            server_id.uuid(),
            user_id.uuid(),
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if !valid {
            return Ok(false);
        }
        sqlx::query!(
            r#"
            INSERT INTO server_member_roles (server_id, user_id, role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
            server_id.uuid(),
            user_id.uuid(),
            role_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn revoke_role(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
        role_id: Uuid,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
            DELETE FROM server_member_roles
            WHERE server_id = $1 AND user_id = $2 AND role_id = $3
            "#,
            server_id.uuid(),
            user_id.uuid(),
            role_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn swap_positions(
        &self,
        server_id: &ServerId,
        role_a: Uuid,
        role_b: Uuid,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
            UPDATE server_roles AS r
            SET position = o.position
            FROM server_roles AS o
            WHERE r.server_id = $1 AND o.server_id = $1
              AND ((r.id = $2 AND o.id = $3) OR (r.id = $3 AND o.id = $2))
            "#,
            server_id.uuid(),
            role_a,
            role_b,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() != 2 {
            return Ok(false);
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
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
