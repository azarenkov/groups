use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SendChannelMessageRequest {
    pub server_id: Uuid,
    pub channel_id: Uuid,
    pub content: String,
    pub ctx: RequestContext,
}

impl SendChannelMessageRequest {
    pub fn new(server_id: Uuid, channel_id: Uuid, content: String, ctx: RequestContext) -> Self {
        Self {
            server_id,
            channel_id,
            content,
            ctx,
        }
    }
}
