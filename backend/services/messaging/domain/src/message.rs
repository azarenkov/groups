pub mod content;
pub mod events;
pub mod id;
pub mod repository;
pub mod view;

use chrono::{DateTime, Utc};

use crate::{channel::id::ChannelId, message::content::MessageContent, message::events::MessageDomainEvent, message::events::lifecycle::MessageDeleted, message::events::lifecycle::MessageEdited, message::events::lifecycle::MessageSent, message::id::MessageId};
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("Message content cannot be empty")]
    ContentEmpty,
    #[error("Message content exceeds maximum length")]
    ContentTooLong,
    #[error("Message content unchanged")]
    ContentUnchanged,
    #[error("Only the message author can perform this action")]
    NotAuthor,
}

#[derive(Debug)]
pub struct Message {
    id: MessageId,
    channel_id: ChannelId,
    author_id: UserId,
    content: MessageContent,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
    events: Vec<MessageDomainEvent>,
}

impl Message {
    pub fn create(
        channel_id: ChannelId,
        author_id: UserId,
        content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        let id = MessageId::generate();
        let mut message = Self {
            id,
            channel_id,
            author_id,
            content: content.clone(),
            created_at: now,
            edited_at: None,
            events: Vec::new(),
        };
        message
            .events
            .push(MessageDomainEvent::Sent(MessageSent::record(
                id, channel_id, author_id, content, now,
            )));
        message
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: MessageId,
        channel_id: ChannelId,
        author_id: UserId,
        content: MessageContent,
        created_at: DateTime<Utc>,
        edited_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            channel_id,
            author_id,
            content,
            created_at,
            edited_at,
            events: Vec::new(),
        }
    }

    pub fn edit(
        &mut self,
        new_content: MessageContent,
        editor_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<(), MessageError> {
        self.edit_with_options(new_content, editor_id, now, false)
    }

    pub fn edit_with_options(
        &mut self,
        new_content: MessageContent,
        editor_id: UserId,
        now: DateTime<Utc>,
        allow_unchanged: bool,
    ) -> Result<(), MessageError> {
        if self.author_id != editor_id {
            return Err(MessageError::NotAuthor);
        }
        if !allow_unchanged && self.content == new_content {
            return Err(MessageError::ContentUnchanged);
        }
        self.content = new_content.clone();
        self.edited_at = Some(now);
        self.events
            .push(MessageDomainEvent::Edited(MessageEdited::record(
                self.id,
                self.channel_id,
                new_content,
                now,
            )));
        Ok(())
    }

    pub fn mark_deleted(
        &mut self,
        deleter_id: UserId,
        is_owner: bool,
        now: DateTime<Utc>,
    ) -> Result<(), MessageError> {
        if self.author_id != deleter_id && !is_owner {
            return Err(MessageError::NotAuthor);
        }
        self.events
            .push(MessageDomainEvent::Deleted(MessageDeleted::record(
                self.id,
                self.channel_id,
                deleter_id,
                now,
            )));
        Ok(())
    }

    pub fn pull_events(&mut self) -> Vec<MessageDomainEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn id(&self) -> &MessageId {
        &self.id
    }

    pub fn channel_id(&self) -> &ChannelId {
        &self.channel_id
    }

    pub fn author_id(&self) -> &UserId {
        &self.author_id
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
pub mod unit_of_work;
