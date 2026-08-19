use application::dto::responses::channel::{
    channel_message::ChannelMessageDto, send_channel_message::SendChannelMessageResponse,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageResponse {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<crate::shared::dto::responses::attachment::AttachmentResponse>,
}

impl From<SendChannelMessageResponse> for MessageResponse {
    fn from(value: SendChannelMessageResponse) -> Self {
        Self {
            id: value.id,
            channel_id: value.channel_id,
            author_id: value.author_id,
            author_name: None,
            content: value.content,
            created_at: value.created_at,
            edited_at: None,
            attachments: value
                .attachments
                .into_iter()
                .map(crate::shared::dto::responses::attachment::AttachmentResponse::from)
                .collect(),
        }
    }
}

impl From<ChannelMessageDto> for MessageResponse {
    fn from(value: ChannelMessageDto) -> Self {
        Self {
            id: value.id,
            channel_id: value.channel_id,
            author_id: value.author_id,
            author_name: value.author_name,
            content: value.content,
            created_at: value.created_at,
            edited_at: value.edited_at,
            attachments: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListChannelMessagesResponse {
    pub messages: Vec<MessageResponse>,
}

impl ListChannelMessagesResponse {
    pub fn new(messages: Vec<MessageResponse>) -> Self {
        Self { messages }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchChannelMessagesResponse {
    pub messages: Vec<MessageResponse>,
}

impl SearchChannelMessagesResponse {
    pub fn new(messages: Vec<MessageResponse>) -> Self {
        Self { messages }
    }
}
