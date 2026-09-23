use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct MuteChannelRequest {
    pub channel_id: Uuid,
    pub community_id: Uuid,
    pub target_user_id: Uuid,
    pub ctx: RequestContext,
}

impl MuteChannelRequest {
    pub fn new(
        channel_id: Uuid,
        community_id: Uuid,
        target_user_id: Uuid,
        ctx: RequestContext,
    ) -> Self {
        Self {
            channel_id,
            community_id,
            target_user_id,
            ctx,
        }
    }
}
