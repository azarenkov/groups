use std::sync::Arc;

use domain::{
    server::{
        id::ServerId,
        repository::ServerRepository,
        role::{AssignOutcome, PERM_ALL, RolePatch, ServerRoleRepository},
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

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        role_id: Uuid,
        patch: RolePatch,
    ) -> Result<(), RoleError> {
        ensure_manage_roles(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
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
        validate_role_input(
            patch.name.as_deref().unwrap_or(&existing.name),
            patch.color.as_deref().unwrap_or(&existing.color),
        )?;
        let projected = (existing.permissions | patch.add_permissions)
            & !patch.remove_permissions
            & PERM_ALL;
        let grantable = actor_grantable_permissions(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        ensure_grantable(projected & !existing.permissions, grantable)?;
        let patch = RolePatch {
            name: patch.name.map(|n| n.trim().to_string()),
            ..patch
        };
        let owner = self.server_repository.find_server_owner(&server_id).await?;
        let delegated = if owner.as_ref() == Some(&actor) {
            None
        } else {
            Some(&actor)
        };
        let members = recipients(self.server_repository.as_ref(), &server_id).await?;
        let event = roles_changed_event(&server_id, &members);
        match self
            .role_repository
            .update_role(&server_id, role_id, &patch, delegated, std::slice::from_ref(&event))
            .await?
        {
            AssignOutcome::Assigned => Ok(()),
            AssignOutcome::NotFound => Err(RoleError::RoleNotFound),
            AssignOutcome::Denied => Err(RoleError::HierarchyViolation),
        }
    }
}
