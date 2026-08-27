use std::sync::Arc;

use chrono::Utc;
use domain::{
    server::{
        id::ServerId,
        repository::ServerRepository,
        role::{CreateOutcome, PERM_ALL, ServerRole, ServerRoleRepository},
    },
    user::id::UserId,
};
use uuid::Uuid;

use super::{
    MAX_ROLES_PER_SERVER, RoleError, actor_grantable_permissions, ensure_grantable,
    ensure_manage_roles, recipients, roles_changed_event, validate_role_input,
};

pub struct CreateRole<S: ServerRepository, R: ServerRoleRepository> {
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: ServerRepository, R: ServerRoleRepository> CreateRole<S, R> {
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
        name: String,
        color: String,
        permissions: i64,
    ) -> Result<ServerRole, RoleError> {
        ensure_manage_roles(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        validate_role_input(&name, &color)?;
        let grantable = actor_grantable_permissions(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        ensure_grantable(permissions & PERM_ALL, grantable)?;
        let owner = self.server_repository.find_server_owner(&server_id).await?;
        let delegated = if owner.as_ref() == Some(&actor) {
            None
        } else {
            Some(&actor)
        };
        let role = ServerRole {
            id: Uuid::new_v4(),
            server_id: server_id.uuid(),
            name: name.trim().to_string(),
            color,
            permissions: permissions & PERM_ALL,
            position: 0,
            created_at: Utc::now(),
        };
        let members = recipients(self.server_repository.as_ref(), &server_id).await?;
        let event = roles_changed_event(&server_id, &members);
        match self
            .role_repository
            .create_role(
                &role,
                delegated,
                MAX_ROLES_PER_SERVER,
                std::slice::from_ref(&event),
            )
            .await?
        {
            CreateOutcome::Created(created) => Ok(created),
            CreateOutcome::LimitReached => Err(RoleError::InvalidData),
            CreateOutcome::Denied => Err(RoleError::HierarchyViolation),
        }
    }
}
