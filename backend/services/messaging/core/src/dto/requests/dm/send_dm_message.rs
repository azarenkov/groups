use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SendDmMessageRequest {
    pub recipient_id: Uuid,
    pub content: String,
    pub attachment_ids: Vec<Uuid>,
    pub reply_to_message_id: Option<Uuid>,
    pub ctx: RequestContext,
}

impl SendDmMessageRequest {
    pub fn new(
        recipient_id: Uuid,
        content: String,
        attachment_ids: Vec<Uuid>,
        reply_to_message_id: Option<Uuid>,
        ctx: RequestContext,
    ) -> Self {
        Self {
            recipient_id,
            content,
            attachment_ids,
            reply_to_message_id,
            ctx,
        }
    }
}
