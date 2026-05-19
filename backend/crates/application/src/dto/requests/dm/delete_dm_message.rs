use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeleteDmMessageRequest {
    pub message_id: Uuid,
    pub ctx: RequestContext,
}

impl DeleteDmMessageRequest {
    pub fn new(message_id: Uuid, ctx: RequestContext) -> Self {
        Self { message_id, ctx }
    }
}
