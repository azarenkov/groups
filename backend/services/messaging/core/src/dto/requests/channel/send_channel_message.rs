use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SendChannelMessageRequest {
    pub community_id: Uuid,
    pub channel_id: Uuid,
    pub content: String,
    pub attachment_ids: Vec<Uuid>,
    pub ctx: RequestContext,
}

impl SendChannelMessageRequest {
    pub fn new(
        community_id: Uuid,
        channel_id: Uuid,
        content: String,
        attachment_ids: Vec<Uuid>,
        ctx: RequestContext,
    ) -> Self {
        Self {
            community_id,
            channel_id,
            content,
            attachment_ids,
            ctx,
        }
    }
}
