use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeafenChannelRequest {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub target_user_id: Uuid,
    pub ctx: RequestContext,
}
