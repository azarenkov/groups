use domain::pagination::Pagination;
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct SearchDmMessagesRequest {
    pub friend_id: Uuid,
    pub query: String,
    pub pagination: Pagination,
    pub ctx: RequestContext,
}

impl SearchDmMessagesRequest {
    pub fn new(
        friend_id: Uuid,
        query: String,
        pagination: Pagination,
        ctx: RequestContext,
    ) -> Self {
        Self {
            friend_id,
            query,
            pagination,
            ctx,
        }
    }
}
