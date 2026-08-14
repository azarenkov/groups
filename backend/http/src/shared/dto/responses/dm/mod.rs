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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_server_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_server_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_duration_seconds: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<crate::shared::dto::responses::attachment::AttachmentResponse>,
}

impl From<SendDmMessageResponse> for DmMessageResponse {
    fn from(value: SendDmMessageResponse) -> Self {
        Self {
            id: value.id,
            author_id: value.sender_id,
            author_name: None,
            recipient_id: value.recipient_id,
            content: value.content,
            invite_server_id: value.invite_server_id,
            invite_server_name: value.invite_server_name,
            call_status: None,
            call_duration_seconds: None,
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

impl From<DmMessageDto> for DmMessageResponse {
    fn from(value: DmMessageDto) -> Self {
        Self {
            id: value.id,
            author_id: value.sender_id,
            author_name: Some(value.sender_name),
            recipient_id: value.recipient_id,
            content: value.content,
            invite_server_id: value.invite_server_id,
            invite_server_name: value.invite_server_name,
            call_status: value.call_status,
            call_duration_seconds: value.call_duration_seconds,
            created_at: value.created_at,
            edited_at: value.edited_at,
            attachments: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptServerInviteResponse {
    pub server_id: Uuid,
}

impl AcceptServerInviteResponse {
    pub fn new(server_id: Uuid) -> Self {
        Self { server_id }
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
