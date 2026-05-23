pub mod events;
pub mod id;
pub mod repository;
pub mod view;

use chrono::{DateTime, Utc};

use crate::{
    dm_message::{
        events::{
            DmMessageDomainEvent,
            lifecycle::{DmMessageDeleted, DmMessageEdited, DmMessageSent},
        },
        id::DmMessageId,
    },
    message::content::MessageContent,
    server::id::ServerId,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum DmMessageError {
    #[error("Only the sender can perform this action")]
    NotSender,
    #[error("Message content unchanged")]
    ContentUnchanged,
    #[error("Server invite messages cannot be edited")]
    NotEditable,
}

#[derive(Debug)]
pub struct DmMessage {
    id: DmMessageId,
    sender_id: UserId,
    recipient_id: UserId,
    content: MessageContent,
    invite_server_id: Option<ServerId>,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
    events: Vec<DmMessageDomainEvent>,
}

impl DmMessage {
    pub fn create(
        sender_id: UserId,
        recipient_id: UserId,
        content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        let id = DmMessageId::generate();
        let mut message = Self {
            id,
            sender_id,
            recipient_id,
            content: content.clone(),
            invite_server_id: None,
            created_at: now,
            edited_at: None,
            events: Vec::new(),
        };
        message.events.push(DmMessageDomainEvent::Sent(
            DmMessageSent::record(id, sender_id, recipient_id, content, now),
        ));
        message
    }

    pub fn create_server_invite(
        sender_id: UserId,
        recipient_id: UserId,
        server_id: ServerId,
        content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        let id = DmMessageId::generate();
        let mut message = Self {
            id,
            sender_id,
            recipient_id,
            content: content.clone(),
            invite_server_id: Some(server_id),
            created_at: now,
            edited_at: None,
            events: Vec::new(),
        };
        message.events.push(DmMessageDomainEvent::Sent(
            DmMessageSent::record(id, sender_id, recipient_id, content, now),
        ));
        message
    }

    pub fn restore(
        id: DmMessageId,
        sender_id: UserId,
        recipient_id: UserId,
        content: MessageContent,
        invite_server_id: Option<ServerId>,
        created_at: DateTime<Utc>,
        edited_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            sender_id,
            recipient_id,
            content,
            invite_server_id,
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
    ) -> Result<(), DmMessageError> {
        if self.sender_id != editor_id {
            return Err(DmMessageError::NotSender);
        }
        if self.invite_server_id.is_some() {
            return Err(DmMessageError::NotEditable);
        }
        if self.content == new_content {
            return Err(DmMessageError::ContentUnchanged);
        }
        self.content = new_content.clone();
        self.edited_at = Some(now);
        self.events.push(DmMessageDomainEvent::Edited(
            DmMessageEdited::record(self.id, self.sender_id, new_content, now),
        ));
        Ok(())
    }

    pub fn mark_deleted(
        &mut self,
        deleter_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<(), DmMessageError> {
        if self.sender_id != deleter_id {
            return Err(DmMessageError::NotSender);
        }
        self.events
            .push(DmMessageDomainEvent::Deleted(DmMessageDeleted::record(
                self.id,
                self.sender_id,
                self.recipient_id,
                now,
            )));
        Ok(())
    }

    pub fn record_invite_deleted(&mut self, now: DateTime<Utc>) {
        self.events
            .push(DmMessageDomainEvent::Deleted(DmMessageDeleted::record(
                self.id,
                self.sender_id,
                self.recipient_id,
                now,
            )));
    }

    pub fn pull_events(&mut self) -> Vec<DmMessageDomainEvent> {
        std::mem::take(&mut self.events)
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
    pub fn invite_server_id(&self) -> Option<&ServerId> {
        self.invite_server_id.as_ref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn edited_at(&self) -> Option<DateTime<Utc>> {
        self.edited_at
    }
}
