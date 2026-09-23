use messaging_domain::community::id::CommunityId;
use domain::user::id::UserId;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeleteCommunityRequest {
    pub community_id: CommunityId,
    pub owner_id: UserId,
    pub ctx: RequestContext,
}
