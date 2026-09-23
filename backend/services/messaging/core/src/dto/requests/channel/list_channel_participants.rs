use domain::pagination::Pagination;
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct ListChannelParticipantsRequest {
    pub community_id: Uuid,
    pub channel_id: Uuid,
    pub requester_id: Uuid,
    pub pagination: Pagination,
    pub ctx: RequestContext,
}

impl ListChannelParticipantsRequest {
    pub fn new(
        community_id: Uuid,
        channel_id: Uuid,
        requester_id: Uuid,
        pagination: Pagination,
        ctx: RequestContext,
    ) -> Self {
        Self {
            community_id,
            channel_id,
            requester_id,
            pagination,
            ctx,
        }
    }
}
