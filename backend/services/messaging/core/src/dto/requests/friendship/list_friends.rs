use messaging_domain::friendship::status::FriendshipStatus;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct ListFriendsRequest {
    pub status: Option<FriendshipStatus>,
    pub ctx: RequestContext,
}

impl ListFriendsRequest {
    pub fn new(status: Option<FriendshipStatus>, ctx: RequestContext) -> Self {
        Self { status, ctx }
    }
}
