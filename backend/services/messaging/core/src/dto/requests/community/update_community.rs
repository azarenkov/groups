use messaging_domain::community::id::CommunityId;
use domain::user::id::UserId;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct UpdateCommunityRequest {
    pub community_id: CommunityId,
    pub owner_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub ctx: RequestContext,
}
