use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct MuteChannelRequest {
    pub channel_id: Uuid,
    pub server_id: Uuid,
    pub target_user_id: Uuid,
    pub ctx: RequestContext,
}

impl MuteChannelRequest {
    pub fn new(
        channel_id: Uuid,
        server_id: Uuid,
        target_user_id: Uuid,
        ctx: RequestContext,
    ) -> Self {
        Self {
            channel_id,
            server_id,
            target_user_id,
            ctx,
        }
    }
}
