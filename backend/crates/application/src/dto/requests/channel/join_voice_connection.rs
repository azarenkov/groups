use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{dto::context::RequestContext, ports::voice_webhook_verifier::WebhookSource};

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
