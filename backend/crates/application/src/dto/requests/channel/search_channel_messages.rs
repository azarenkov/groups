use domain::shared::pagination::Pagination;
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SearchChannelMessagesRequest {
    pub server_id: Uuid,
    pub channel_id: Uuid,
    pub query: String,
    pub pagination: Pagination,
    pub ctx: RequestContext,
}

impl SearchChannelMessagesRequest {
    pub fn new(
        server_id: Uuid,
        channel_id: Uuid,
        query: String,
        pagination: Pagination,
        ctx: RequestContext,
    ) -> Self {
        Self {
            server_id,
            channel_id,
            query,
            pagination,
            ctx,
        }
    }
}
