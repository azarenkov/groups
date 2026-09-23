use messaging_domain::{channel::id::ChannelId, community::id::CommunityId};
use domain::user::id::UserId;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeleteChannelRequest {
    pub community_id: CommunityId,
    pub channel_id: ChannelId,
    pub deleted_by: UserId,
    pub ctx: RequestContext,
}
