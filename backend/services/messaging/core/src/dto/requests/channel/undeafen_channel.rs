use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct UndeafenChannelRequest {
    pub channel_id: Uuid,
    pub server_id: Uuid,
    pub target_user_id: Uuid,
    pub ctx: RequestContext,
}
