use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct EditDmMessageRequest {
    pub message_id: Uuid,
    pub content: String,
    pub add_attachment_ids: Vec<Uuid>,
    pub remove_attachment_ids: Vec<Uuid>,
    pub ctx: RequestContext,
}

impl EditDmMessageRequest {
    pub fn new(
        message_id: Uuid,
        content: String,
        add_attachment_ids: Vec<Uuid>,
        remove_attachment_ids: Vec<Uuid>,
        ctx: RequestContext,
    ) -> Self {
        Self {
            message_id,
            content,
            add_attachment_ids,
            remove_attachment_ids,
            ctx,
        }
    }
}
