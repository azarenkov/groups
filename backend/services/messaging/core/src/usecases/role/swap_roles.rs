use std::sync::Arc;

use domain::{
    server::{
        id::ServerId,
        repository::ServerRepository,
        role::{AssignOutcome, ServerRoleRepository},
    },
    user::id::UserId,
};
use uuid::Uuid;

use super::{RoleError, actor_height, ensure_below, ensure_manage_roles, recipients, roles_changed_event};

pub struct SwapRoles<S: ServerRepository, R: ServerRoleRepository> {
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: ServerRepository, R: ServerRoleRepository> SwapRoles<S, R> {
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
        role_a: Uuid,
        role_b: Uuid,
    ) -> Result<(), RoleError> {
        if role_a == role_b {
            return Err(RoleError::InvalidData);
        }
        ensure_manage_roles(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        let roles = self.role_repository.list_roles(&server_id).await?;
        let a = roles.iter().find(|r| r.id == role_a).ok_or(RoleError::RoleNotFound)?;
        let b = roles.iter().find(|r| r.id == role_b).ok_or(RoleError::RoleNotFound)?;
        let height = actor_height(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        ensure_below(&height, a.position)?;
        ensure_below(&height, b.position)?;
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
            .swap_positions(
                &server_id,
                role_a,
                role_b,
                delegated,
                std::slice::from_ref(&event),
            )
            .await?
        {
            AssignOutcome::Assigned => Ok(()),
            AssignOutcome::NotFound => Err(RoleError::RoleNotFound),
            AssignOutcome::Denied => Err(RoleError::HierarchyViolation),
        }
    }
}
