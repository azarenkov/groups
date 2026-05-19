use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct EditDmMessageRequest {
    pub message_id: Uuid,
    pub content: String,
    pub ctx: RequestContext,
}

impl EditDmMessageRequest {
    pub fn new(message_id: Uuid, content: String, ctx: RequestContext) -> Self {
        Self {
            message_id,
            content,
            ctx,
        }
    }
}
