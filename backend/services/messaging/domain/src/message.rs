pub mod content;
pub mod events;
pub mod id;
pub mod reply;
pub mod repository;
pub mod system;
pub mod view;

use chrono::{DateTime, Utc};

use crate::{channel::id::ChannelId, message::reply::ReplyPreview, message::content::MessageContent, message::events::MessageDomainEvent, message::events::lifecycle::MessageDeleted, message::events::lifecycle::MessageEdited, message::events::lifecycle::MessageSent, message::id::MessageId, message::system::MessageKind, message::system::SystemMessage, message::system::SystemMessageKind};
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
    #[error("System messages cannot be modified")]
    SystemMessage,
}

#[derive(Debug)]
pub struct Message {
    id: MessageId,
    channel_id: ChannelId,
    author_id: UserId,
    content: MessageContent,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
    system: Option<SystemMessage>,
    reply: Option<ReplyPreview>,
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
            system: None,
            reply: None,
            events: Vec::new(),
        };
        message
            .events
            .push(MessageDomainEvent::Sent(MessageSent::record(
                id, channel_id, author_id, content, now,
            )));
        message
    }

    pub fn system_notice(
        channel_id: ChannelId,
        subject: UserId,
        kind: SystemMessageKind,
        now: DateTime<Utc>,
    ) -> Self {
        let id = MessageId::generate();
        let content = MessageContent::empty();
        let system = SystemMessage {
            kind,
            user_id: subject,
        };
        let mut sent = MessageSent::record(id, channel_id, subject, content.clone(), now);
        sent.system = Some(system);
        Self {
            id,
            channel_id,
            author_id: subject,
            content,
            created_at: now,
            edited_at: None,
            system: Some(system),
            reply: None,
            events: vec![MessageDomainEvent::Sent(sent)],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: MessageId,
        channel_id: ChannelId,
        author_id: UserId,
        content: MessageContent,
        created_at: DateTime<Utc>,
        edited_at: Option<DateTime<Utc>>,
        system: Option<SystemMessage>,
    ) -> Self {
        Self {
            id,
            channel_id,
            author_id,
            content,
            created_at,
            edited_at,
            system,
            reply: None,
            events: Vec::new(),
        }
    }

    pub fn with_reply(mut self, reply: ReplyPreview) -> Self {
        for event in &mut self.events {
            if let MessageDomainEvent::Sent(sent) = event {
                sent.reply = Some(reply.clone());
            }
        }
        self.reply = Some(reply);
        self
    }

    pub fn reply(&self) -> Option<&ReplyPreview> {
        self.reply.as_ref()
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
        if self.system.is_some() {
            return Err(MessageError::SystemMessage);
        }
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
        if self.system.is_some() {
            return Err(MessageError::SystemMessage);
        }
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

    pub fn kind(&self) -> MessageKind {
        if self.system.is_some() {
            MessageKind::System
        } else {
            MessageKind::User
        }
    }

    pub fn system(&self) -> Option<&SystemMessage> {
        self.system.as_ref()
    }
}
pub mod unit_of_work;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::system::SystemMessageKind;
    use uuid::Uuid;

    #[test]
    fn system_notice_carries_subject_and_rejects_edits_and_deletion() {
        let channel = ChannelId::new(Uuid::new_v4());
        let subject = UserId::new(Uuid::new_v4());
        let mut message = Message::system_notice(channel, subject, SystemMessageKind::MemberJoined, Utc::now());
        assert_eq!(message.kind(), MessageKind::System);
        assert_eq!(message.system().map(|s| s.user_id), Some(subject));
        assert_eq!(message.content().as_str(), "");
        let events = message.pull_events();
        assert!(matches!(events.as_slice(), [MessageDomainEvent::Sent(sent)] if sent.system.map(|s| s.kind) == Some(SystemMessageKind::MemberJoined)));
        let content = MessageContent::new("hello".to_string()).unwrap();
        assert!(matches!(message.edit(content, subject, Utc::now()), Err(MessageError::SystemMessage)));
        assert!(matches!(message.mark_deleted(subject, true, Utc::now()), Err(MessageError::SystemMessage)));
    }

    #[test]
    fn user_messages_keep_the_user_kind() {
        let channel = ChannelId::new(Uuid::new_v4());
        let author = UserId::new(Uuid::new_v4());
        let message = Message::create(channel, author, MessageContent::new("hi".to_string()).unwrap(), Utc::now());
        assert_eq!(message.kind(), MessageKind::User);
        assert!(message.system().is_none());
    }
}
