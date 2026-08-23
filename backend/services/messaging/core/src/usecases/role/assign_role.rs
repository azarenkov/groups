use std::sync::Arc;

use domain::{
    server::{id::ServerId, repository::ServerRepository, role::ServerRoleRepository},
    user::id::UserId,
};
use uuid::Uuid;

use super::{RoleError, ensure_manage_roles, recipients, roles_changed_event};

pub struct AssignRole<S: ServerRepository, R: ServerRoleRepository> {
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: ServerRepository, R: ServerRoleRepository> AssignRole<S, R> {
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
        target: UserId,
        role_id: Uuid,
    ) -> Result<(), RoleError> {
        ensure_manage_roles(
            self.server_repository.as_ref(),
            self.role_repository.as_ref(),
            &server_id,
            &actor,
        )
        .await?;
        let members = recipients(self.server_repository.as_ref(), &server_id).await?;
        let event = roles_changed_event(&server_id, &members);
        let assigned = self
            .role_repository
            .assign_role(&server_id, &target, role_id, std::slice::from_ref(&event))
            .await?;
        if !assigned {
            return Err(RoleError::RoleNotFound);
        }
        Ok(())
    }
}
