use chrono::{DateTime, Utc};
use uuid::Uuid;

use domain::channel::webhook_source::WebhookSource;

use crate::dto::context::RequestContext;

#[derive(Debug)]
pub struct JoinVoiceConnectionRequest {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub session_id: String,
    pub occurred_at: DateTime<Utc>,
    pub inbox_source: WebhookSource,
    pub inbox_external_id: String,
    pub ctx: RequestContext,
}
