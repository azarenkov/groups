use std::sync::Arc;

use domain::{
    server::{
        id::ServerId,
        repository::ServerRepository,
        role::{ServerRole, ServerRoleRepository},
    },
    user::id::UserId,
};
use uuid::Uuid;

use super::{RoleError, ensure_member};

pub struct RolesOverview {
    pub roles: Vec<ServerRole>,
    pub assignments: Vec<(Uuid, Uuid)>,
}

pub struct ListRoles<S: ServerRepository, R: ServerRoleRepository> {
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: ServerRepository, R: ServerRoleRepository> ListRoles<S, R> {
    pub fn new(server_repository: Arc<S>, role_repository: Arc<R>) -> Self {
        Self {
            server_repository,
            role_repository,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
    ) -> Result<RolesOverview, RoleError> {
        ensure_member(self.server_repository.as_ref(), &server_id, &user_id).await?;
        let roles = self.role_repository.list_roles(&server_id).await?;
        let assignments = self.role_repository.member_role_ids(&server_id).await?;
        Ok(RolesOverview { roles, assignments })
    }
}
