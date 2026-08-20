use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct RemoveFriendshipRequest {
    pub other_user_id: Uuid,
    pub ctx: RequestContext,
}

impl RemoveFriendshipRequest {
    pub fn new(other_user_id: Uuid, ctx: RequestContext) -> Self {
        Self { other_user_id, ctx }
    }
}
