use messaging_domain::server::id::ServerId;
use domain::user::id::UserId;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct UpdateServerRequest {
    pub server_id: ServerId,
    pub owner_id: UserId,
    pub name: String,
    pub description: Option<String>,
    pub ctx: RequestContext,
}
