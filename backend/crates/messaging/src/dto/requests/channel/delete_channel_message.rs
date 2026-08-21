use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeleteChannelMessageRequest {
    pub server_id: Uuid,
    pub channel_id: Uuid,
    pub message_id: Uuid,
    pub ctx: RequestContext,
}

impl DeleteChannelMessageRequest {
    pub fn new(server_id: Uuid, channel_id: Uuid, message_id: Uuid, ctx: RequestContext) -> Self {
        Self {
            server_id,
            channel_id,
            message_id,
            ctx,
        }
    }
}
