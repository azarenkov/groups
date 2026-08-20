use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SendDmMessageRequest {
    pub recipient_id: Uuid,
    pub content: String,
    pub attachment_ids: Vec<Uuid>,
    pub ctx: RequestContext,
}

impl SendDmMessageRequest {
    pub fn new(
        recipient_id: Uuid,
        content: String,
        attachment_ids: Vec<Uuid>,
        ctx: RequestContext,
    ) -> Self {
        Self {
            recipient_id,
            content,
            attachment_ids,
            ctx,
        }
    }
}
