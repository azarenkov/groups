use messaging_domain::server::id::ServerId;
use domain::user::id::UserId;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeleteServerRequest {
    pub server_id: ServerId,
    pub owner_id: UserId,
    pub ctx: RequestContext,
}
