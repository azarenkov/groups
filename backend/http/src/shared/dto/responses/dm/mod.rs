use application::dto::responses::dm::{
    dm_message::DmMessageDto, send_dm_message::SendDmMessageResponse,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmMessageResponse {
    pub id: Uuid,
    pub author_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    pub recipient_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

impl From<SendDmMessageResponse> for DmMessageResponse {
    fn from(value: SendDmMessageResponse) -> Self {
        Self {
            id: value.id,
            author_id: value.sender_id,
            author_name: None,
            recipient_id: value.recipient_id,
            content: value.content,
            created_at: value.created_at,
            edited_at: None,
        }
    }
}

impl From<DmMessageDto> for DmMessageResponse {
    fn from(value: DmMessageDto) -> Self {
        Self {
            id: value.id,
            author_id: value.sender_id,
            author_name: Some(value.sender_name),
            recipient_id: value.recipient_id,
            content: value.content,
            created_at: value.created_at,
            edited_at: value.edited_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListDmMessagesResponse {
    pub messages: Vec<DmMessageResponse>,
}

impl ListDmMessagesResponse {
    pub fn new(messages: Vec<DmMessageResponse>) -> Self {
        Self { messages }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchDmMessagesResponse {
    pub messages: Vec<DmMessageResponse>,
}

impl SearchDmMessagesResponse {
    pub fn new(messages: Vec<DmMessageResponse>) -> Self {
        Self { messages }
    }
}
