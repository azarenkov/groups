use std::sync::Arc;

use messaging_domain::{community::id::CommunityId, community::repository::CommunityRepository, community::role::PERM_MANAGE_SERVER, community::role::CommunityRoleRepository, community::role::has_permission};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

pub struct CanManageCommunity<S: CommunityRepository, R: CommunityRoleRepository> {
    community_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<S: CommunityRepository, R: CommunityRoleRepository> CanManageCommunity<S, R> {
    pub fn new(community_repository: Arc<S>, role_repository: Arc<R>) -> Self {
        Self {
            community_repository,
            role_repository,
        }
    }

    pub async fn execute(&self, community_id: CommunityId, user_id: UserId) -> Result<bool, RepositoryError> {
        let owner = self.community_repository.find_community_owner(&community_id).await?;
        if owner.as_ref() == Some(&user_id) {
            return Ok(true);
        }
        if !self.community_repository.is_community_member(&community_id, &user_id).await? {
            return Ok(false);
        }
        let perms = self.role_repository.user_permissions(&community_id, &user_id).await?;
        Ok(has_permission(perms, PERM_MANAGE_SERVER))
    }
}
