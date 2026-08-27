use messaging_domain::{channel::id::ChannelId, server::id::ServerId};
use domain::user::id::UserId;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct DeleteChannelRequest {
    pub server_id: ServerId,
    pub channel_id: ChannelId,
    pub deleted_by: UserId,
    pub ctx: RequestContext,
}
