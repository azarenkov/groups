use async_trait::async_trait;
use domain::{community::id::CommunityId, user::id::UserId};

#[derive(Debug, thiserror::Error)]
pub enum CommunityOwnerLookupError {
    #[error("Owner lookup unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait CommunityOwnerLookup {
    async fn find_owner(
        &self,
        community_id: &CommunityId,
    ) -> Result<Option<UserId>, CommunityOwnerLookupError>;

    async fn can_manage(
        &self,
        community_id: &CommunityId,
        user_id: &UserId,
    ) -> Result<bool, CommunityOwnerLookupError>;
}
