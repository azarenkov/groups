use chrono::{DateTime, Utc};
use messaging_domain::channel::events::participant::MediaSource;
use uuid::Uuid;

use contracts::events::webhook::WebhookSource;

use crate::dto::context::RequestContext;

#[derive(Debug)]
pub struct TrackMediaConnectionRequest {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub session_id: String,
    pub source: MediaSource,
    pub published: bool,
    pub occurred_at: DateTime<Utc>,
    pub inbox_source: WebhookSource,
    pub inbox_external_id: String,
    pub ctx: RequestContext,
}
