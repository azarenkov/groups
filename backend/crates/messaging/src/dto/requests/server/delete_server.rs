use domain::{server::id::ServerId, user::id::UserId};

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeleteServerRequest {
    pub server_id: ServerId,
    pub owner_id: UserId,
    pub ctx: RequestContext,
}
