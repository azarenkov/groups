use chrono::{DateTime, Utc};
use messaging_domain::message::{Message, system::SystemMessage, view::MessageView};
use uuid::Uuid;

#[derive(Debug)]
pub struct SystemMessageDto {
    pub kind: String,
    pub user_id: Uuid,
}

impl From<&SystemMessage> for SystemMessageDto {
    fn from(value: &SystemMessage) -> Self {
        Self {
            kind: value.kind.as_str().to_string(),
            user_id: value.user_id.uuid(),
        }
    }
}

#[derive(Debug)]
pub struct ChannelMessageDto {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub author_name: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub kind: String,
    pub system: Option<SystemMessageDto>,
}

impl From<Message> for ChannelMessageDto {
    fn from(value: Message) -> Self {
        Self {
            id: value.id().uuid(),
            channel_id: value.channel_id().uuid(),
            author_id: value.author_id().uuid(),
            author_name: None,
            content: value.content().as_str().to_string(),
            created_at: value.created_at(),
            edited_at: value.edited_at(),
            kind: value.kind().as_str().to_string(),
            system: value.system().map(SystemMessageDto::from),
        }
    }
}

impl From<MessageView> for ChannelMessageDto {
    fn from(value: MessageView) -> Self {
        Self {
            id: value.id.uuid(),
            channel_id: value.channel_id.uuid(),
            author_id: value.author_id.uuid(),
            author_name: Some(value.author_name),
            content: value.content,
            created_at: value.created_at,
            edited_at: value.edited_at,
            kind: if value.system.is_some() { "system" } else { "user" }.to_string(),
            system: value.system.as_ref().map(SystemMessageDto::from),
        }
    }
}
