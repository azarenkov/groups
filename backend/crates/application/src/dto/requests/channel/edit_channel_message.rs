use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct EditChannelMessageRequest {
    pub server_id: Uuid,
    pub channel_id: Uuid,
    pub message_id: Uuid,
    pub new_content: String,
    pub ctx: RequestContext,
}

impl EditChannelMessageRequest {
    pub fn new(
        server_id: Uuid,
        channel_id: Uuid,
        message_id: Uuid,
        new_content: String,
        ctx: RequestContext,
    ) -> Self {
        Self {
            server_id,
            channel_id,
            message_id,
            new_content,
            ctx,
        }
    }
}
