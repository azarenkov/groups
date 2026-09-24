pub mod events;
pub mod id;
pub mod repository;
pub mod view;

use chrono::{DateTime, Utc};

use crate::{message::reply::ReplyPreview, dm_message::events::DmMessageDomainEvent, dm_message::events::lifecycle::DmMessageDeleted, dm_message::events::lifecycle::DmMessageEdited, dm_message::events::lifecycle::DmMessageSent, dm_message::id::DmMessageId, message::content::MessageContent, community::id::CommunityId};
use domain::user::id::UserId;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DmMessageError {
    #[error("Only the sender can perform this action")]
    NotSender,
    #[error("Message content unchanged")]
    ContentUnchanged,
    #[error("Community invite messages cannot be edited")]
    NotEditable,
}

#[derive(Debug)]
pub struct DmMessage {
    id: DmMessageId,
    sender_id: UserId,
    recipient_id: UserId,
    content: MessageContent,
    invite_community_id: Option<CommunityId>,
    call_status: Option<String>,
    call_duration_seconds: Option<i32>,
    call_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
    reply: Option<ReplyPreview>,
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
            invite_community_id: None,
            call_status: None,
            call_duration_seconds: None,
            call_id: None,
            created_at: now,
            edited_at: None,
            reply: None,
            events: Vec::new(),
        };
        message.events.push(DmMessageDomainEvent::Sent(
            DmMessageSent::record(id, sender_id, recipient_id, content, now),
        ));
        message
    }

    pub fn create_call(
        sender_id: UserId,
        recipient_id: UserId,
        call_id: Uuid,
        call_status: String,
        call_duration_seconds: Option<i32>,
        content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        let id = DmMessageId::generate();
        let mut message = Self {
            id,
            sender_id,
            recipient_id,
            content: content.clone(),
            invite_community_id: None,
            call_status: Some(call_status),
            call_duration_seconds,
            call_id: Some(call_id),
            created_at: now,
            edited_at: None,
            reply: None,
            events: Vec::new(),
        };
        message.events.push(DmMessageDomainEvent::Sent(
            DmMessageSent::record(id, sender_id, recipient_id, content, now)
                .for_call(message.call_status.as_deref().unwrap_or_default(), call_duration_seconds),
        ));
        message
    }

    pub fn create_community_invite(
        sender_id: UserId,
        recipient_id: UserId,
        community_id: CommunityId,
        content: MessageContent,
        now: DateTime<Utc>,
    ) -> Self {
        let id = DmMessageId::generate();
        let mut message = Self {
            id,
            sender_id,
            recipient_id,
            content: content.clone(),
            invite_community_id: Some(community_id),
            call_status: None,
            call_duration_seconds: None,
            call_id: None,
            created_at: now,
            edited_at: None,
            reply: None,
            events: Vec::new(),
        };
        let community_name = content.as_str().to_string();
        message.events.push(DmMessageDomainEvent::Sent(
            DmMessageSent::record(id, sender_id, recipient_id, content, now)
                .for_invite(community_id, &community_name),
        ));
        message
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_reply(mut self, reply: ReplyPreview) -> Self {
        for event in &mut self.events {
            if let DmMessageDomainEvent::Sent(sent) = event {
                sent.reply = Some(reply.clone());
            }
        }
        self.reply = Some(reply);
        self
    }

    pub fn reply(&self) -> Option<&ReplyPreview> {
        self.reply.as_ref()
    }

    pub fn restore(
        id: DmMessageId,
        sender_id: UserId,
        recipient_id: UserId,
        content: MessageContent,
        invite_community_id: Option<CommunityId>,
        call_status: Option<String>,
        call_duration_seconds: Option<i32>,
        created_at: DateTime<Utc>,
        edited_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            sender_id,
            recipient_id,
            content,
            invite_community_id,
            call_status,
            call_duration_seconds,
            call_id: None,
            created_at,
            edited_at,
            reply: None,
            events: Vec::new(),
        }
    }

    pub fn edit(
        &mut self,
        new_content: MessageContent,
        editor_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<(), DmMessageError> {
        self.edit_with_options(new_content, editor_id, now, false)
    }

    pub fn edit_with_options(
        &mut self,
        new_content: MessageContent,
        editor_id: UserId,
        now: DateTime<Utc>,
        allow_unchanged: bool,
    ) -> Result<(), DmMessageError> {
        if self.sender_id != editor_id {
            return Err(DmMessageError::NotSender);
        }
        if self.invite_community_id.is_some() {
            return Err(DmMessageError::NotEditable);
        }
        if !allow_unchanged && self.content == new_content {
            return Err(DmMessageError::ContentUnchanged);
        }
        self.content = new_content.clone();
        self.edited_at = Some(now);
        self.events.push(DmMessageDomainEvent::Edited(
            DmMessageEdited::record(self.id, self.sender_id, self.recipient_id, new_content, now),
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
    pub fn invite_community_id(&self) -> Option<&CommunityId> {
        self.invite_community_id.as_ref()
    }
    pub fn call_status(&self) -> Option<&str> {
        self.call_status.as_deref()
    }
    pub fn call_duration_seconds(&self) -> Option<i32> {
        self.call_duration_seconds
    }
    pub fn call_id(&self) -> Option<Uuid> {
        self.call_id
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn edited_at(&self) -> Option<DateTime<Utc>> {
        self.edited_at
    }
}
pub mod unit_of_work;
