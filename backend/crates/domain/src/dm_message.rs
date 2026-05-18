pub mod id;
pub mod repository;
pub mod view;

use chrono::{DateTime, Utc};

use crate::{
    dm_message::id::DmMessageId, message::content::MessageContent, user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum DmMessageError {
    #[error("Only the sender can perform this action")]
    NotSender,
    #[error("Message content unchanged")]
    ContentUnchanged,
}

#[derive(Debug, Clone)]
pub struct DmMessage {
    id: DmMessageId,
    sender_id: UserId,
    recipient_id: UserId,
    content: MessageContent,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
}

impl DmMessage {
    pub fn create(
        sender_id: UserId,
        recipient_id: UserId,
        content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: DmMessageId::generate(),
            sender_id,
            recipient_id,
            content,
            created_at: now,
            edited_at: None,
        }
    }

    pub fn restore(
        id: DmMessageId,
        sender_id: UserId,
        recipient_id: UserId,
        content: MessageContent,
        created_at: DateTime<Utc>,
        edited_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            sender_id,
            recipient_id,
            content,
            created_at,
            edited_at,
        }
    }

    pub fn edit(
        &mut self,
        new_content: MessageContent,
        editor_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<(), DmMessageError> {
        if self.sender_id != editor_id {
            return Err(DmMessageError::NotSender);
        }
        if self.content == new_content {
            return Err(DmMessageError::ContentUnchanged);
        }
        self.content = new_content;
        self.edited_at = Some(now);
        Ok(())
    }

    pub fn can_delete(&self, deleter_id: UserId) -> Result<(), DmMessageError> {
        if self.sender_id != deleter_id {
            return Err(DmMessageError::NotSender);
        }
        Ok(())
    }

    pub fn id(&self) -> &DmMessageId {
        &self.id
    }
    pub fn sender_id(&self) -> &UserId {
        &self.sender_id
    }
    pub fn recipient_id(&self) -> &UserId {
        &self.recipient_id
    }
    pub fn content(&self) -> &MessageContent {
        &self.content
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn edited_at(&self) -> Option<DateTime<Utc>> {
        self.edited_at
    }
}
