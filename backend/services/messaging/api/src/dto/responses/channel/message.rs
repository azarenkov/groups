use messaging::dto::responses::channel::{
    channel_message::ChannelMessageDto, channel_message::SystemMessageDto,
    send_channel_message::SendChannelMessageResponse,
};
use chrono::{DateTime, Utc};
use crate::dto::responses::reply::{RepliesTo, ReplyPreviewResponse};
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
    pub attachments: Vec<api_common::dto::responses::attachment::AttachmentResponse>,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemMessageResponse>,
    #[serde(skip)]
    pub reply_to_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<ReplyPreviewResponse>,
}

impl RepliesTo for MessageResponse {
    fn reply_to_id(&self) -> Option<Uuid> {
        self.reply_to_id
    }
    fn set_reply(&mut self, reply: ReplyPreviewResponse) {
        self.reply_to = Some(reply);
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMessageResponse {
    #[serde(rename = "type")]
    pub kind: String,
    pub user_id: Uuid,
}

impl From<SystemMessageDto> for SystemMessageResponse {
    fn from(value: SystemMessageDto) -> Self {
        Self {
            kind: value.kind,
            user_id: value.user_id,
        }
    }
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
                .map(api_common::dto::responses::attachment::AttachmentResponse::from)
                .collect(),
            kind: "user".to_string(),
            system: None,
            reply_to_id: value.reply.as_ref().map(|r| r.id),
            reply_to: value.reply.map(ReplyPreviewResponse::from),
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
            kind: value.kind,
            system: value.system.map(SystemMessageResponse::from),
            reply_to_id: value.reply_to_message_id,
            reply_to: None,
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
