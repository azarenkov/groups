use application::dto::responses::channel::send_channel_message::SendChannelMessageResponse;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageResponse {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

impl From<SendChannelMessageResponse> for MessageResponse {
    fn from(value: SendChannelMessageResponse) -> Self {
        Self {
            id: value.id,
            channel_id: value.channel_id,
            author_id: value.author_id,
            content: value.content,
            created_at: value.created_at,
            edited_at: None,
        }
    }
}
