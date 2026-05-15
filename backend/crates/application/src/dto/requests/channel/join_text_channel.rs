use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct JoinTextChannelRequest {
    pub server_id: Uuid,
    pub channel_id: Uuid,
    pub ctx: RequestContext,
}

impl JoinTextChannelRequest {
    pub fn new(server_id: Uuid, channel_id: Uuid, ctx: RequestContext) -> Self {
        Self {
            server_id,
            channel_id,
            ctx,
        }
    }
}
