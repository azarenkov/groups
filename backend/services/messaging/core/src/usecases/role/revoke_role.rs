use std::sync::Arc;

use domain::{
    server::{id::ServerId, repository::ServerRepository, role::ServerRoleRepository},
    user::id::UserId,
};
use uuid::Uuid;

use super::{RoleError, actor_height, ensure_below, ensure_manage_roles, recipients, roles_changed_event};

pub struct RevokeRole<S: ServerRepository, R: ServerRoleRepository> {
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: ServerRepository, R: ServerRoleRepository> RevokeRole<S, R> {
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
        let target_role = self
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
        ensure_below(&height, target_role.position)?;
        let members = recipients(self.server_repository.as_ref(), &server_id).await?;
        let event = roles_changed_event(&server_id, &members);
        let revoked = self
            .role_repository
            .revoke_role(&server_id, &target, role_id, std::slice::from_ref(&event))
            .await?;
        if !revoked {
            return Err(RoleError::RoleNotFound);
        }
        Ok(())
    }
}
