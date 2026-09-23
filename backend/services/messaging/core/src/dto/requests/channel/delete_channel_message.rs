use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeleteChannelMessageRequest {
    pub community_id: Uuid,
    pub channel_id: Uuid,
    pub message_id: Uuid,
    pub ctx: RequestContext,
}

impl DeleteChannelMessageRequest {
    pub fn new(community_id: Uuid, channel_id: Uuid, message_id: Uuid, ctx: RequestContext) -> Self {
        Self {
            community_id,
            channel_id,
            message_id,
            ctx,
        }
    }
}
