use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SendFriendRequestRequest {
    pub target_username: String,
    pub ctx: RequestContext,
}

impl SendFriendRequestRequest {
    pub fn new(target_username: String, ctx: RequestContext) -> Self {
        Self {
            target_username,
            ctx,
        }
    }
}
