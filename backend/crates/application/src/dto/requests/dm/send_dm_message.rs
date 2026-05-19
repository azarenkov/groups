use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SendDmMessageRequest {
    pub recipient_id: Uuid,
    pub content: String,
    pub ctx: RequestContext,
}

impl SendDmMessageRequest {
    pub fn new(recipient_id: Uuid, content: String, ctx: RequestContext) -> Self {
        Self {
            recipient_id,
            content,
            ctx,
        }
    }
}
