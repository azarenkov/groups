use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct ListChannelMessagesRequest {
    pub community_id: Uuid,
    pub channel_id: Uuid,
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: i64,
    pub ctx: RequestContext,
}

impl ListChannelMessagesRequest {
    pub fn new(
        community_id: Uuid,
        channel_id: Uuid,
        before: Option<Uuid>,
        after: Option<Uuid>,
        limit: i64,
        ctx: RequestContext,
    ) -> Self {
        Self {
            community_id,
            channel_id,
            before,
            after,
            limit,
            ctx,
        }
    }
}
