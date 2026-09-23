use std::sync::Arc;

use messaging_domain::{community::id::CommunityId, community::repository::CommunityRepository, community::role::CommunityRole, community::role::CommunityRoleRepository};
use domain::user::id::UserId;
use uuid::Uuid;

use super::{RoleError, ensure_member};

pub struct RolesOverview {
    pub roles: Vec<CommunityRole>,
    pub assignments: Vec<(Uuid, Uuid)>,
}

pub struct ListRoles<S: CommunityRepository, R: CommunityRoleRepository> {
    community_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: CommunityRepository, R: CommunityRoleRepository> ListRoles<S, R> {
    pub fn new(community_repository: Arc<S>, role_repository: Arc<R>) -> Self {
        Self {
            community_repository,
            role_repository,
        }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        user_id: UserId,
    ) -> Result<RolesOverview, RoleError> {
        ensure_member(self.community_repository.as_ref(), &community_id, &user_id).await?;
        let roles = self.role_repository.list_roles(&community_id).await?;
        let assignments = self.role_repository.member_role_ids(&community_id).await?;
        Ok(RolesOverview { roles, assignments })
    }
}
