use std::sync::Arc;

use domain::{
    server::{
        id::ServerId,
        repository::ServerRepository,
        role::{PERM_ALL, ServerRole, ServerRoleRepository},
    },
    user::id::UserId,
};
use uuid::Uuid;

use super::{RoleError, actor_grantable_permissions, actor_height, ensure_below, ensure_grantable, ensure_manage_roles, recipients, roles_changed_event, validate_role_input};

pub struct UpdateRole<S: ServerRepository, R: ServerRoleRepository> {
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: ServerRepository, R: ServerRoleRepository> UpdateRole<S, R> {
    pub fn new(server_repository: Arc<S>, role_repository: Arc<R>) -> Self {
        Self {
            server_repository,
            role_repository,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        role_id: Uuid,
        name: String,
        color: String,
        permissions: i64,
        position: Option<i32>,
    ) -> Result<(), RoleError> {
        ensure_manage_roles(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        validate_role_input(&name, &color)?;
        let existing = self
            .role_repository
            .list_roles(&server_id)
            .await?
            .into_iter()
            .find(|r| r.id == role_id)
            .ok_or(RoleError::RoleNotFound)?;
        let height = actor_height(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        ensure_below(&height, existing.position)?;
        if let Some(p) = position {
            ensure_below(&height, p)?;
        }
        let grantable = actor_grantable_permissions(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        ensure_grantable(
            (permissions & PERM_ALL) & !existing.permissions,
            grantable,
        )?;
        let role = ServerRole {
            name: name.trim().to_string(),
            color,
            permissions: permissions & PERM_ALL,
            position: position.unwrap_or(existing.position),
            ..existing
        };
        let members = recipients(self.server_repository.as_ref(), &server_id).await?;
        let event = roles_changed_event(&server_id, &members);
        let updated = self
            .role_repository
            .update_role(&role, std::slice::from_ref(&event))
            .await?;
        if !updated {
            return Err(RoleError::RoleNotFound);
        }
        Ok(())
    }
}
