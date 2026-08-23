use std::sync::Arc;

use chrono::Utc;
use domain::{
    server::{
        id::ServerId,
        repository::ServerRepository,
        role::{PERM_ALL, ServerRole, ServerRoleRepository},
    },
    user::id::UserId,
};
use uuid::Uuid;

use super::{
    ActorHeight, MAX_ROLES_PER_SERVER, RoleError, actor_grantable_permissions, actor_height,
    ensure_grantable, ensure_manage_roles, recipients, roles_changed_event, validate_role_input,
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
        let existing = self.role_repository.list_roles(&server_id).await?;
        if existing.len() >= MAX_ROLES_PER_SERVER {
            return Err(RoleError::InvalidData);
        }
        let height = actor_height(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        let position = match height {
            ActorHeight::Owner => existing.iter().map(|r| r.position).max().unwrap_or(0) + 1,
            ActorHeight::Position(top) => {
                if top == i32::MIN {
                    return Err(RoleError::HierarchyViolation);
                }
                top - 1
            }
        };
        let role = ServerRole {
            id: Uuid::new_v4(),
            server_id: server_id.uuid(),
            name: name.trim().to_string(),
            color,
            permissions: permissions & PERM_ALL,
            position,
            created_at: Utc::now(),
        };
        let members = recipients(self.server_repository.as_ref(), &server_id).await?;
        let event = roles_changed_event(&server_id, &members);
        self.role_repository
            .create_role(&role, std::slice::from_ref(&event))
            .await?;
        Ok(role)
    }
}
