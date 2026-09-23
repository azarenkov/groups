use domain::pagination::Pagination;
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SearchChannelMessagesRequest {
    pub community_id: Uuid,
    pub channel_id: Uuid,
    pub query: String,
    pub pagination: Pagination,
    pub ctx: RequestContext,
}

impl SearchChannelMessagesRequest {
    pub fn new(
        community_id: Uuid,
        channel_id: Uuid,
        query: String,
        pagination: Pagination,
        ctx: RequestContext,
    ) -> Self {
        Self {
            community_id,
            channel_id,
            query,
            pagination,
            ctx,
        }
    }
}
