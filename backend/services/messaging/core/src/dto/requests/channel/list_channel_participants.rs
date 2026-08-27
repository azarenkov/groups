use domain::pagination::Pagination;
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct ListChannelParticipantsRequest {
    pub server_id: Uuid,
    pub channel_id: Uuid,
    pub requester_id: Uuid,
    pub pagination: Pagination,
    pub ctx: RequestContext,
}

impl ListChannelParticipantsRequest {
    pub fn new(
        server_id: Uuid,
        channel_id: Uuid,
        requester_id: Uuid,
        pagination: Pagination,
        ctx: RequestContext,
    ) -> Self {
        Self {
            server_id,
            channel_id,
            requester_id,
            pagination,
            ctx,
        }
    }
}
