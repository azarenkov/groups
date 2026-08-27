use std::collections::HashSet;

use async_trait::async_trait;
use domain::{
    server::{
        authorization::AuthorizationSnapshot,
        id::ServerId,
        role::ServerRole,
        unit_of_work::{ServerTransaction, ServerUnitOfWork},
    },
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresServerUnitOfWork {
    pool: Pool<Postgres>,
}

impl PostgresServerUnitOfWork {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub struct PostgresServerTransaction {
    tx: Transaction<'static, Postgres>,
    server_id: ServerId,
}

pub async fn lock_server(
    tx: &mut Transaction<'_, Postgres>,
    server_id: &ServerId,
) -> Result<(), RepositoryError> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1::text, 42))",
        server_id.uuid().to_string(),
    )
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    Ok(())
}

pub async fn load_authorization(
    tx: &mut Transaction<'_, Postgres>,
    server_id: &ServerId,
) -> Result<AuthorizationSnapshot, RepositoryError> {
    let owner = sqlx::query_scalar!(
        r#"SELECT owner_id FROM servers WHERE id = $1"#,
        server_id.uuid(),
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .ok_or(RepositoryError::NotFound)?;
    let roles = sqlx::query!(
        r#"
        SELECT id, server_id, name, color, permissions, position, created_at
        FROM server_roles
        WHERE server_id = $1
        ORDER BY position DESC, created_at ASC
        "#,
        server_id.uuid(),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
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
    .collect();
    let assignments = sqlx::query!(
        r#"
        SELECT user_id, role_id FROM server_member_roles WHERE server_id = $1
        "#,
        server_id.uuid(),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .into_iter()
    .map(|r| (UserId::new(r.user_id), r.role_id))
    .collect();
    let members: HashSet<UserId> = sqlx::query_scalar!(
        r#"SELECT user_id FROM server_members WHERE server_id = $1"#,
        server_id.uuid(),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .into_iter()
    .map(UserId::new)
    .collect();
    Ok(AuthorizationSnapshot {
        owner_id: UserId::new(owner),
        roles,
        assignments,
        members,
    })
}

#[async_trait]
impl ServerUnitOfWork for PostgresServerUnitOfWork {
    async fn begin(&self, server_id: &ServerId) -> Result<Box<dyn ServerTransaction>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        lock_server(&mut tx, server_id).await?;
        Ok(Box::new(PostgresServerTransaction {
            tx,
            server_id: *server_id,
        }))
    }
}

#[async_trait]
impl ServerTransaction for PostgresServerTransaction {
    async fn authorization(&mut self) -> Result<AuthorizationSnapshot, RepositoryError> {
        load_authorization(&mut self.tx, &self.server_id).await
    }

    async fn insert_role(&mut self, role: &ServerRole) -> Result<(), RepositoryError> {
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
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn update_role(
        &mut self,
        role_id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
        permissions: i64,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            UPDATE server_roles
            SET name = COALESCE($3, name), color = COALESCE($4, color), permissions = $5
            WHERE id = $1 AND server_id = $2
            "#,
            role_id,
            self.server_id.uuid(),
            name,
            color,
            permissions,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_role(&mut self, role_id: Uuid) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"DELETE FROM server_roles WHERE id = $1 AND server_id = $2"#,
            role_id,
            self.server_id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn assign_role(&mut self, user_id: &UserId, role_id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO server_member_roles (server_id, user_id, role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
            self.server_id.uuid(),
            user_id.uuid(),
            role_id,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn revoke_role(&mut self, user_id: &UserId, role_id: Uuid) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM server_member_roles
            WHERE server_id = $1 AND user_id = $2 AND role_id = $3
            "#,
            self.server_id.uuid(),
            user_id.uuid(),
            role_id,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn swap_role_positions(&mut self, role_a: Uuid, role_b: Uuid) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            UPDATE server_roles AS r
            SET position = o.position
            FROM server_roles AS o
            WHERE r.server_id = $1 AND o.server_id = $1
              AND ((r.id = $2 AND o.id = $3) OR (r.id = $3 AND o.id = $2))
            "#,
            self.server_id.uuid(),
            role_a,
            role_b,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() == 2)
    }

    async fn save_outbox(&mut self, records: &[OutboxRecord]) -> Result<(), RepositoryError> {
        save_outbox_records(&mut self.tx, records).await
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}
