use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct AcceptFriendRequestRequest {
    pub sender_id: Uuid,
    pub ctx: RequestContext,
}

impl AcceptFriendRequestRequest {
    pub fn new(sender_id: Uuid, ctx: RequestContext) -> Self {
        Self { sender_id, ctx }
    }
}
