pub mod call;
pub mod events;
pub mod id;
pub mod repository;
pub mod view;

use chrono::{DateTime, Utc};

use crate::{dm_message::call::CallStatus, message::forward::ForwardInfo, message::reply::ReplyPreview, dm_message::events::DmMessageDomainEvent, dm_message::events::lifecycle::DmMessageDeleted, dm_message::events::lifecycle::DmMessageEdited, dm_message::events::lifecycle::DmMessageSent, dm_message::id::DmMessageId, message::content::MessageContent};
use domain::{sticker::StickerSnapshot, user::id::UserId};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DmMessageError {
    #[error("Only the sender can perform this action")]
    NotSender,
    #[error("Message content unchanged")]
    ContentUnchanged,
    #[error("Forwarded messages cannot be edited")]
    Forwarded,
    #[error("Sticker messages cannot be edited")]
    Sticker,
}

#[derive(Debug)]
pub struct DmMessage {
    id: DmMessageId,
    sender_id: UserId,
    recipient_id: UserId,
    content: MessageContent,
    call_status: Option<String>,
    call_duration_seconds: Option<i32>,
    call_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
    reply: Option<ReplyPreview>,
    forward: Option<ForwardInfo>,
    sticker: Option<StickerSnapshot>,
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
            call_status: None,
            call_duration_seconds: None,
            call_id: None,
            created_at: now,
            edited_at: None,
            reply: None,
            forward: None,
            sticker: None,
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
        status: CallStatus,
        content: MessageContent,
        now: DateTime<Utc>,
        recipient_read_at: Option<DateTime<Utc>>,
    ) -> Self {
        let call_status = status.as_str().to_string();
        let call_duration_seconds = status.duration_seconds();
        let id = DmMessageId::generate();
        let mut message = Self {
            id,
            sender_id,
            recipient_id,
            content: content.clone(),
            call_status: Some(call_status),
            call_duration_seconds,
            call_id: Some(call_id),
            created_at: now,
            edited_at: None,
            reply: None,
            forward: None,
            sticker: None,
            events: Vec::new(),
        };
        message.events.push(DmMessageDomainEvent::Sent(
            DmMessageSent::record(id, sender_id, recipient_id, content, now)
                .for_call(
                    message.call_status.as_deref().unwrap_or_default(),
                    call_duration_seconds,
                    CallStatus::predates(now, recipient_read_at),
                ),
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

    pub fn with_forward(mut self, forward: ForwardInfo) -> Self {
        for event in &mut self.events {
            if let DmMessageDomainEvent::Sent(sent) = event {
                sent.forward = Some(forward.clone());
            }
        }
        self.forward = Some(forward);
        self
    }

    pub fn forward(&self) -> Option<&ForwardInfo> {
        self.forward.as_ref()
    }

    pub fn with_sticker(mut self, sticker: Option<StickerSnapshot>) -> Self {
        for event in &mut self.events {
            if let DmMessageDomainEvent::Sent(sent) = event {
                sent.sticker = sticker.clone();
            }
        }
        self.sticker = sticker;
        self
    }

    pub fn hide_for(&mut self, users: Vec<UserId>) {
        for event in &mut self.events {
            if let DmMessageDomainEvent::Sent(sent) = event {
                sent.hidden_for = users.clone();
            }
        }
    }

    pub fn set_reply_hidden_for(&mut self, users: Vec<UserId>) {
        for event in &mut self.events {
            if let DmMessageDomainEvent::Sent(sent) = event
                && let Some(reply) = sent.reply.as_mut()
            {
                reply.hidden_for = users.clone();
            }
        }
        if let Some(reply) = self.reply.as_mut() {
            reply.hidden_for = users;
        }
    }

    pub fn restamp(&mut self, now: DateTime<Utc>) {
        for event in &mut self.events {
            if let DmMessageDomainEvent::Sent(sent) = event {
                sent.occurred_at = now;
            }
        }
        self.created_at = now;
    }

    pub fn sticker(&self) -> Option<&StickerSnapshot> {
        self.sticker.as_ref()
    }

    pub fn restore(
        id: DmMessageId,
        sender_id: UserId,
        recipient_id: UserId,
        content: MessageContent,
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
            call_status,
            call_duration_seconds,
            call_id: None,
            created_at,
            edited_at,
            reply: None,
            forward: None,
            sticker: None,
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
        if self.forward.is_some() {
            return Err(DmMessageError::Forwarded);
        }
        if self.sticker.is_some() {
            return Err(DmMessageError::Sticker);
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn restamp_moves_the_message_and_its_sent_event() {
        let sent_at = Utc::now();
        let mut message = DmMessage::create(
            UserId::generate(),
            UserId::generate(),
            MessageContent::new("hi".to_string()).unwrap(),
            sent_at,
        );
        let later = sent_at + Duration::milliseconds(5);
        message.restamp(later);
        assert_eq!(message.created_at(), later);
        let events = message.pull_events();
        assert!(matches!(&events[..], [DmMessageDomainEvent::Sent(sent)] if sent.occurred_at == later));
    }

    #[test]
    fn reply_hidden_for_updates_the_message_and_its_sent_event() {
        let peer = UserId::generate();
        let mut message = DmMessage::create(
            UserId::generate(),
            peer,
            MessageContent::new("hi".to_string()).unwrap(),
            Utc::now(),
        )
        .with_reply(ReplyPreview::new(uuid::Uuid::new_v4(), peer, "old", false));
        message.set_reply_hidden_for(vec![peer]);
        assert_eq!(message.reply().unwrap().hidden_for, vec![peer]);
        let events = message.pull_events();
        assert!(matches!(&events[..], [DmMessageDomainEvent::Sent(sent)] if sent.reply.as_ref().unwrap().hidden_for == vec![peer]));
    }
}
