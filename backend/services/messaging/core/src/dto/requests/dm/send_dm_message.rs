use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SendDmMessageRequest {
    pub recipient_id: Uuid,
    pub content: String,
    pub attachment_ids: Vec<Uuid>,
    pub reply_to_message_id: Option<Uuid>,
    pub forward: Option<crate::dto::requests::forward::ForwardedContent>,
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
            forward: None,
            ctx,
        }
    }

    pub fn with_forward(mut self, forward: crate::dto::requests::forward::ForwardedContent) -> Self {
        self.reply_to_message_id = None;
        self.attachment_ids = Vec::new();
        self.forward = Some(forward);
        self
    }
}
