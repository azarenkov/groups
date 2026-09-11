use domain::pagination::KeysetCursor;
use messaging_domain::friendship::status::FriendshipStatus;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct ListFriendsRequest {
    pub status: Option<FriendshipStatus>,
    pub cursor: Option<KeysetCursor>,
    pub limit: i64,
    pub ctx: RequestContext,
}

impl ListFriendsRequest {
    pub fn new(
        status: Option<FriendshipStatus>,
        cursor: Option<KeysetCursor>,
        limit: i64,
        ctx: RequestContext,
    ) -> Self {
        Self {
            status,
            cursor,
            limit,
            ctx,
        }
    }
}
