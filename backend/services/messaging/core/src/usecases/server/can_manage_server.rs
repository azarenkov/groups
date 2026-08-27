use std::sync::Arc;

use messaging_domain::{server::id::ServerId, server::repository::ServerRepository, server::role::PERM_MANAGE_SERVER, server::role::ServerRoleRepository, server::role::has_permission};
use domain::{shared::repository::error::RepositoryError, user::id::UserId};

pub struct CanManageServer<S: ServerRepository, R: ServerRoleRepository> {
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: ServerRepository, R: ServerRoleRepository> CanManageServer<S, R> {
    pub fn new(server_repository: Arc<S>, role_repository: Arc<R>) -> Self {
        Self {
            server_repository,
            role_repository,
        }
    }

    pub async fn execute(&self, server_id: ServerId, user_id: UserId) -> Result<bool, RepositoryError> {
        let owner = self.server_repository.find_server_owner(&server_id).await?;
        if owner.as_ref() == Some(&user_id) {
            return Ok(true);
        }
        if !self.server_repository.is_server_member(&server_id, &user_id).await? {
            return Ok(false);
        }
        let perms = self.role_repository.user_permissions(&server_id, &user_id).await?;
        Ok(has_permission(perms, PERM_MANAGE_SERVER))
    }
}
