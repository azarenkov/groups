use uuid::Uuid;

use crate::dto::context::RequestContext;

#[derive(Debug)]
pub struct UpdateStreamViewerRequest {
    pub channel_id: Uuid,
    pub streamer_id: Uuid,
    pub viewer_session_id: String,
    pub watching: bool,
    pub ctx: RequestContext,
}
