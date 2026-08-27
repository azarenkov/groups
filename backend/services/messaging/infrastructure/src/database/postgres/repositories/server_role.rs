use async_trait::async_trait;
use domain::{
    server::{
        id::ServerId,
        role::{AssignOutcome, CreateOutcome, PERM_ALL, RolePatch, ServerRole, ServerRoleRepository},
    },
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

struct ActorState {
    top: Option<i32>,
    perms: i64,
}

async fn lock_server(
    tx: &mut sqlx::Transaction<'_, Postgres>,
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

async fn actor_state(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    server_id: &ServerId,
    actor: &UserId,
) -> Result<ActorState, RepositoryError> {
    let row = sqlx::query!(
        r#"
        SELECT max(r.position) AS "top", bit_or(r.permissions) AS "perms"
        FROM server_member_roles mr
        INNER JOIN server_roles r ON r.id = mr.role_id
        INNER JOIN server_members m
            ON m.server_id = mr.server_id AND m.user_id = mr.user_id
        WHERE mr.server_id = $1 AND mr.user_id = $2
        "#,
        server_id.uuid(),
        actor.uuid(),
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    Ok(ActorState {
        top: row.top,
        perms: row.perms.unwrap_or(0),
    })
}

fn delegated_ok(state: &ActorState, positions: &[i32], added_perms: i64) -> bool {
    use domain::server::role::{PERM_MANAGE_ROLES, has_permission};
    if !has_permission(state.perms, PERM_MANAGE_ROLES) {
        return false;
    }
    let Some(top) = state.top else { return false };
    positions.iter().all(|p| *p < top) && added_perms & !state.perms == 0
}

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
        delegated_actor: Option<&UserId>,
        max_roles: usize,
        events: &[OutboxRecord],
    ) -> Result<CreateOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let server_id = ServerId::new(role.server_id);
        lock_server(&mut tx, &server_id).await?;
        let taken: Vec<i32> = sqlx::query_scalar!(
            r#"
            SELECT position FROM server_roles WHERE server_id = $1
            "#,
            role.server_id,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if taken.len() >= max_roles {
            return Ok(CreateOutcome::LimitReached);
        }
        let position = match delegated_actor {
            None => taken.iter().copied().max().unwrap_or(0) + 1,
            Some(actor) => {
                let state = actor_state(&mut tx, &server_id, actor).await?;
                if !delegated_ok(&state, &[], role.permissions) {
                    return Ok(CreateOutcome::Denied);
                }
                let Some(top) = state.top else {
                    return Ok(CreateOutcome::Denied);
                };
                let set: std::collections::HashSet<i32> = taken.into_iter().collect();
                let mut candidate = top - 1;
                while set.contains(&candidate) {
                    candidate -= 1;
                }
                candidate
            }
        };
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
            position,
            role.created_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(CreateOutcome::Created(ServerRole {
            position,
            ..role.clone()
        }))
    }

    async fn update_role(
        &self,
        server_id: &ServerId,
        role_id: Uuid,
        patch: &RolePatch,
        delegated_actor: Option<&UserId>,
        events: &[OutboxRecord],
    ) -> Result<AssignOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let server_id = *server_id;
        lock_server(&mut tx, &server_id).await?;
        let existing = sqlx::query!(
            r#"
            SELECT position, permissions FROM server_roles
            WHERE id = $1 AND server_id = $2
            "#,
            role_id,
            server_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let Some(existing) = existing else {
            return Ok(AssignOutcome::NotFound);
        };
        let new_permissions =
            (existing.permissions | patch.add_permissions) & !patch.remove_permissions & PERM_ALL;
        if let Some(actor) = delegated_actor {
            let state = actor_state(&mut tx, &server_id, actor).await?;
            let added = new_permissions & !existing.permissions;
            if !delegated_ok(&state, &[existing.position], added) {
                return Ok(AssignOutcome::Denied);
            }
            let removed = existing.permissions & !new_permissions;
            if removed != 0 {
                let owner_holds = sqlx::query_scalar!(
                    r#"
                    SELECT EXISTS(
                        SELECT 1
                        FROM server_member_roles mr
                        INNER JOIN servers s ON s.id = mr.server_id
                        WHERE mr.server_id = $1 AND mr.role_id = $2 AND mr.user_id = s.owner_id
                    ) AS "exists!"
                    "#,
                    server_id.uuid(),
                    role_id,
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                if owner_holds {
                    return Ok(AssignOutcome::Denied);
                }
                let highest_assignee = sqlx::query_scalar!(
                    r#"
                    SELECT max(top.position) AS "top"
                    FROM server_member_roles holder
                    INNER JOIN server_member_roles mr
                        ON mr.server_id = holder.server_id AND mr.user_id = holder.user_id
                    INNER JOIN server_roles top ON top.id = mr.role_id
                    WHERE holder.server_id = $1 AND holder.role_id = $2
                    "#,
                    server_id.uuid(),
                    role_id,
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                let outranks_all = match (state.top, highest_assignee) {
                    (Some(a), Some(t)) => a > t,
                    (Some(_), None) => true,
                    _ => false,
                };
                if !outranks_all {
                    return Ok(AssignOutcome::Denied);
                }
            }
        }
        sqlx::query!(
            r#"
            UPDATE server_roles
            SET name = COALESCE($3, name), color = COALESCE($4, color), permissions = $5
            WHERE id = $1 AND server_id = $2
            "#,
            role_id,
            server_id.uuid(),
            patch.name.as_deref(),
            patch.color.as_deref(),
            new_permissions,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(AssignOutcome::Assigned)
    }

    async fn delete_role(
        &self,
        server_id: &ServerId,
        role_id: Uuid,
        delegated_actor: Option<&UserId>,
        events: &[OutboxRecord],
    ) -> Result<AssignOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        lock_server(&mut tx, server_id).await?;
        let existing = sqlx::query!(
            r#"
            SELECT position FROM server_roles
            WHERE id = $1 AND server_id = $2
            "#,
            role_id,
            server_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let Some(existing) = existing else {
            return Ok(AssignOutcome::NotFound);
        };
        if let Some(actor) = delegated_actor {
            let state = actor_state(&mut tx, server_id, actor).await?;
            if !delegated_ok(&state, &[existing.position], 0) {
                return Ok(AssignOutcome::Denied);
            }
            let owner_holds = sqlx::query_scalar!(
                r#"
                SELECT EXISTS(
                    SELECT 1
                    FROM server_member_roles mr
                    INNER JOIN servers s ON s.id = mr.server_id
                    WHERE mr.server_id = $1 AND mr.role_id = $2 AND mr.user_id = s.owner_id
                ) AS "exists!"
                "#,
                server_id.uuid(),
                role_id,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            if owner_holds {
                return Ok(AssignOutcome::Denied);
            }
            let highest_assignee = sqlx::query_scalar!(
                r#"
                SELECT max(top.position) AS "top"
                FROM server_member_roles holder
                INNER JOIN server_member_roles mr
                    ON mr.server_id = holder.server_id AND mr.user_id = holder.user_id
                INNER JOIN server_roles top ON top.id = mr.role_id
                WHERE holder.server_id = $1 AND holder.role_id = $2
                "#,
                server_id.uuid(),
                role_id,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            let outranks_all = match (state.top, highest_assignee) {
                (Some(a), Some(t)) => a > t,
                (Some(_), None) => true,
                _ => false,
            };
            if !outranks_all {
                return Ok(AssignOutcome::Denied);
            }
        }
        sqlx::query!(
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
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(AssignOutcome::Assigned)
    }

    async fn assign_role(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
        role_id: Uuid,
        delegated_actor: Option<&UserId>,
        events: &[OutboxRecord],
    ) -> Result<AssignOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            "SELECT pg_advisory_xact_lock(hashtextextended($1::text, 42))",
            server_id.uuid().to_string(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let role = sqlx::query!(
            r#"
            SELECT r.position, r.permissions
            FROM server_roles r
            INNER JOIN server_members m
                ON m.server_id = r.server_id AND m.user_id = $3
            WHERE r.id = $1 AND r.server_id = $2
            "#,
            role_id,
            server_id.uuid(),
            user_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let Some(role) = role else {
            return Ok(AssignOutcome::NotFound);
        };
        if let Some(actor) = delegated_actor {
            let state = actor_state(&mut tx, server_id, actor).await?;
            if !delegated_ok(&state, &[role.position], role.permissions) {
                return Ok(AssignOutcome::Denied);
            }
            let target_is_owner = sqlx::query_scalar!(
                r#"SELECT owner_id FROM servers WHERE id = $1"#,
                server_id.uuid(),
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .is_some_and(|owner| owner == user_id.uuid());
            if target_is_owner {
                return Ok(AssignOutcome::Denied);
            }
            let target_top = sqlx::query_scalar!(
                r#"
                SELECT max(r.position) AS "top"
                FROM server_member_roles mr
                INNER JOIN server_roles r ON r.id = mr.role_id
                WHERE mr.server_id = $1 AND mr.user_id = $2
                "#,
                server_id.uuid(),
                user_id.uuid(),
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            let outranks = match (state.top, target_top) {
                (Some(a), Some(t)) => a > t,
                (Some(_), None) => true,
                _ => false,
            };
            if !outranks {
                return Ok(AssignOutcome::Denied);
            }
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
        Ok(AssignOutcome::Assigned)
    }

    async fn revoke_role(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
        role_id: Uuid,
        delegated_actor: Option<&UserId>,
        events: &[OutboxRecord],
    ) -> Result<AssignOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        lock_server(&mut tx, server_id).await?;
        let existing = sqlx::query!(
            r#"
            SELECT position FROM server_roles
            WHERE id = $1 AND server_id = $2
            "#,
            role_id,
            server_id.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let Some(existing) = existing else {
            return Ok(AssignOutcome::NotFound);
        };
        if let Some(actor) = delegated_actor {
            let state = actor_state(&mut tx, server_id, actor).await?;
            if !delegated_ok(&state, &[existing.position], 0) {
                return Ok(AssignOutcome::Denied);
            }
            let target_is_owner = sqlx::query_scalar!(
                r#"SELECT owner_id FROM servers WHERE id = $1"#,
                server_id.uuid(),
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?
            .is_some_and(|owner| owner == user_id.uuid());
            if target_is_owner {
                return Ok(AssignOutcome::Denied);
            }
            let target_top = sqlx::query_scalar!(
                r#"
                SELECT max(r.position) AS "top"
                FROM server_member_roles mr
                INNER JOIN server_roles r ON r.id = mr.role_id
                WHERE mr.server_id = $1 AND mr.user_id = $2
                "#,
                server_id.uuid(),
                user_id.uuid(),
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            let outranks = match (state.top, target_top) {
                (Some(a), Some(t)) => a > t,
                (Some(_), None) => true,
                _ => false,
            };
            if !outranks {
                return Ok(AssignOutcome::Denied);
            }
        }
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
            return Ok(AssignOutcome::NotFound);
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(AssignOutcome::Assigned)
    }

    async fn swap_positions(
        &self,
        server_id: &ServerId,
        role_a: Uuid,
        role_b: Uuid,
        delegated_actor: Option<&UserId>,
        events: &[OutboxRecord],
    ) -> Result<AssignOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        lock_server(&mut tx, server_id).await?;
        if let Some(actor) = delegated_actor {
            let rows = sqlx::query!(
                r#"
                SELECT position FROM server_roles
                WHERE server_id = $1 AND id = ANY(ARRAY[$2, $3]::uuid[])
                "#,
                server_id.uuid(),
                role_a,
                role_b,
            )
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            if rows.len() != 2 {
                return Ok(AssignOutcome::NotFound);
            }
            let positions: Vec<i32> = rows.iter().map(|r| r.position).collect();
            let state = actor_state(&mut tx, server_id, actor).await?;
            if !delegated_ok(&state, &positions, 0) {
                return Ok(AssignOutcome::Denied);
            }
        }
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
            return Ok(AssignOutcome::NotFound);
        }
        save_outbox_records(&mut tx, events).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(AssignOutcome::Assigned)
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
