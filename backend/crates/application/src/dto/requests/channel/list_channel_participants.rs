use domain::shared::pagination::Pagination;
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct ListChannelParticipantsRequest {
    pub channel_id: Uuid,
    pub pagination: Pagination,
    pub ctx: RequestContext,
}

impl ListChannelParticipantsRequest {
    pub fn new(channel_id: Uuid, pagination: Pagination, ctx: RequestContext) -> Self {
        Self {
            channel_id,
            pagination,
            ctx,
        }
    }
}
