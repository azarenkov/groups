use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct ListDmMessagesRequest {
    pub friend_id: Uuid,
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: i64,
    pub ctx: RequestContext,
}

impl ListDmMessagesRequest {
    pub fn new(
        friend_id: Uuid,
        before: Option<Uuid>,
        after: Option<Uuid>,
        limit: i64,
        ctx: RequestContext,
    ) -> Self {
        Self {
            friend_id,
            before,
            after,
            limit,
            ctx,
        }
    }
}
