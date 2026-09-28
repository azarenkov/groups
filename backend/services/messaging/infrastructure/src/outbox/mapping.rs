use contracts::events::{
    channel::{
        ChannelCreatedV1, ChannelDeletedV1, ChannelRenamedV1, ParticipantConnectionAbortedV1,
        ParticipantDeafenedV1, ParticipantJoinedV1, ParticipantLeftV1, ParticipantMediaV1,
        ParticipantMutedV1, ParticipantUndeafenedV1, ParticipantUnmutedV1, TextChannelJoinedV1,
    },
    dm_message::{DmMessageDeletedV1, DmMessageEditedV1, DmMessageSentV1},
    friendship::{FriendRequestAcceptedV1, FriendRequestSentV1, FriendshipRemovedV1, UserBlockedV1, UserUnblockedV1},
    message::{ChannelMessageDeletedV1, ChannelMessageEditedV1, ChannelMessageSentV1},
    community::{CommunityCreatedV1, CommunityDeletedV1, CommunityOwnerChangedV1, CommunityMemberV1, CommunityRolesChangedV1, CommunityUpdatedV1},
};
use domain::user::id::UserId;
use messaging_domain::{channel::events::ChannelEvent, dm_message::events::DmMessageDomainEvent, events::MessagingEvent, friendship::events::FriendshipDomainEvent, message::events::MessageDomainEvent, community::events::CommunityEvent};
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

pub fn to_records(events: &[MessagingEvent], request_id: Option<&str>) -> Result<Vec<OutboxRecord>, RepositoryError> {
    events.iter().map(|e| to_record(e, request_id)).collect()
}

pub fn to_record(event: &MessagingEvent, request_id: Option<&str>) -> Result<OutboxRecord, RepositoryError> {
    let mut record = match event {
        MessagingEvent::Channel(e) => channel_record(e)?,
        MessagingEvent::Community(e) => community_record(e)?,
        MessagingEvent::Message(e) => message_record(e)?,
        MessagingEvent::DmMessage(e) => dm_record(e)?,
        MessagingEvent::Friendship(e) => friendship_record(e)?,
    };
    record.request_id = request_id.map(str::to_string);
    Ok(record)
}

fn channel_record(event: &ChannelEvent) -> Result<OutboxRecord, RepositoryError> {
    let payload = match event {
        ChannelEvent::Created(e) => encode(&ChannelCreatedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            owner_id: e.owner_id.uuid(),
            name: value(&e.name)?,
            kind: value(&e.kind)?,
            visibility: value(&e.visibility)?,
            occurred_at: e.occurred_at,
            recipient_ids: uuids(&e.recipient_ids),
            is_default: e.is_default,
        })?,
        ChannelEvent::Renamed(e) => encode(&ChannelRenamedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            old_name: value(&e.old_name)?,
            new_name: value(&e.new_name)?,
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::Deleted(e) => encode(&ChannelDeletedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            deleted_by: e.deleted_by.uuid(),
            occurred_at: e.occurred_at,
            recipient_ids: uuids(&e.recipient_ids),
        })?,
        ChannelEvent::ParticipantJoined(e) => encode(&ParticipantJoinedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::ParticipantLeft(e) => encode(&ParticipantLeftV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            reason: value(&e.reason)?,
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::ParticipantMuted(e) => encode(&ParticipantMutedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            muted_by: e.muted_by.uuid(),
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::ParticipantUnmuted(e) => encode(&ParticipantUnmutedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            unmuted_by: e.unmuted_by.uuid(),
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::ParticipantDeafened(e) => encode(&ParticipantDeafenedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            deafened_by: e.deafened_by.uuid(),
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::ParticipantUndeafened(e) => encode(&ParticipantUndeafenedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            undeafened_by: e.undeafened_by.uuid(),
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::ParticipantConnectionAborted(e) => encode(&ParticipantConnectionAbortedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            reason: value(&e.reason)?,
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::ParticipantMediaChanged(e) => encode(&ParticipantMediaV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            source: snake_case(&e.source)?,
            enabled: e.enabled,
            occurred_at: e.occurred_at,
        })?,
        ChannelEvent::TextJoined(e) => encode(&TextChannelJoinedV1 {
            channel_id: e.channel_id.uuid(),
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            joined_at: e.joined_at,
            recipient_ids: uuids(&e.recipient_ids),
        })?,
    };
    Ok(record(
        event.channel_id().uuid(),
        AggregateType::Channel,
        Some((event.community_id().uuid(), AggregateType::Community)),
        event.event_type(),
        payload,
        event.occurred_at(),
    ))
}

fn community_record(event: &CommunityEvent) -> Result<OutboxRecord, RepositoryError> {
    let payload = match event {
        CommunityEvent::Created(e) => encode(&CommunityCreatedV1 {
            community_id: e.community_id.uuid(),
            owner_id: e.owner_id.uuid(),
            name: value(&e.name)?,
            description: e.description.as_ref().map(value).transpose()?,
            occurred_at: e.occurred_at,
        })?,
        CommunityEvent::Updated(e) => encode(&CommunityUpdatedV1 {
            community_id: e.community_id.uuid(),
            name: value(&e.name)?,
            description: e.description.as_ref().map(value).transpose()?,
            member_ids: uuids(&e.member_ids),
            occurred_at: e.occurred_at,
        })?,
        CommunityEvent::Deleted(e) => encode(&CommunityDeletedV1 {
            community_id: e.community_id.uuid(),
            owner_id: e.owner_id.uuid(),
            member_ids: Some(uuids(&e.member_ids)),
            occurred_at: e.occurred_at,
        })?,
        CommunityEvent::MemberJoined(e) => encode(&CommunityMemberV1 {
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
        CommunityEvent::MemberLeft(e) => encode(&CommunityMemberV1 {
            community_id: e.community_id.uuid(),
            user_id: e.user_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
        CommunityEvent::RolesChanged(e) => encode(&CommunityRolesChangedV1 {
            community_id: e.community_id.uuid(),
            recipients: uuids(&e.recipients),
        })?,
        CommunityEvent::OwnerChanged(e) => encode(&CommunityOwnerChangedV1 {
            community_id: e.community_id.uuid(),
            owner_id: e.owner_id.uuid(),
            previous_owner_id: e.previous_owner_id.uuid(),
            member_ids: uuids(&e.member_ids),
            occurred_at: e.occurred_at,
        })?,
    };
    Ok(record(
        event.community_id().uuid(),
        AggregateType::Community,
        None,
        event.event_type(),
        payload,
        event.occurred_at(),
    ))
}

fn message_record(event: &MessageDomainEvent) -> Result<OutboxRecord, RepositoryError> {
    let payload = match event {
        MessageDomainEvent::Sent(e) => encode(&ChannelMessageSentV1 {
            message_id: e.message_id.uuid(),
            channel_id: e.channel_id.uuid(),
            author_id: e.author_id.uuid(),
            content: value(&e.content)?,
            occurred_at: e.occurred_at,
            recipient_ids: uuids(&e.recipient_ids),
            attachments: Some(value(&e.attachments)?),
            mentions: Some(uuids(&e.mentions)),
            system: e.system.as_ref().map(|s| s.to_json()),
            reply_to: e.reply.as_ref().map(reply_json),
            forwarded_from: e.forward.as_ref().map(|f| f.to_json()),
        })?,
        MessageDomainEvent::Edited(e) => encode(&ChannelMessageEditedV1 {
            message_id: e.message_id.uuid(),
            channel_id: e.channel_id.uuid(),
            new_content: value(&e.new_content)?,
            occurred_at: e.occurred_at,
            recipient_ids: uuids(&e.recipient_ids),
            attachments: Some(value(&e.attachments)?),
        })?,
        MessageDomainEvent::Deleted(e) => encode(&ChannelMessageDeletedV1 {
            message_id: e.message_id.uuid(),
            channel_id: e.channel_id.uuid(),
            deleted_by: e.deleted_by.uuid(),
            occurred_at: e.occurred_at,
            recipient_ids: uuids(&e.recipient_ids),
        })?,
    };
    Ok(record(
        event.message_id().uuid(),
        AggregateType::Message,
        Some((event.channel_id().uuid(), AggregateType::Channel)),
        event.event_type(),
        payload,
        event.occurred_at(),
    ))
}

fn dm_record(event: &DmMessageDomainEvent) -> Result<OutboxRecord, RepositoryError> {
    let payload = match event {
        DmMessageDomainEvent::Sent(e) => encode(&DmMessageSentV1 {
            message_id: e.message_id.uuid(),
            sender_id: e.sender_id.uuid(),
            recipient_id: e.recipient_id.uuid(),
            content: value(&e.content)?,
            occurred_at: e.occurred_at,
            invite_community_id: e.invite_community_id.map(|s| s.uuid()),
            invite_community_name: e.invite_community_name.clone(),
            call_status: e.call_status.clone(),
            call_duration_seconds: e.call_duration_seconds,
            attachments: Some(value(&e.attachments)?),
            reply_to: e.reply.as_ref().map(reply_json),
            forwarded_from: e.forward.as_ref().map(|f| f.to_json()),
        })?,
        DmMessageDomainEvent::Edited(e) => encode(&DmMessageEditedV1 {
            message_id: e.message_id.uuid(),
            sender_id: e.sender_id.uuid(),
            new_content: value(&e.new_content)?,
            occurred_at: e.occurred_at,
            recipient_id: Some(e.recipient_id.uuid()),
            attachments: Some(value(&e.attachments)?),
            hidden_for: uuids(&e.hidden_for),
        })?,
        DmMessageDomainEvent::Deleted(e) => encode(&DmMessageDeletedV1 {
            message_id: e.message_id.uuid(),
            sender_id: e.sender_id.uuid(),
            recipient_id: e.recipient_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
    };
    Ok(record(
        event.aggregate_id(),
        AggregateType::DmMessage,
        event.parent_id().map(|u| (u.uuid(), AggregateType::User)),
        event.event_type(),
        payload,
        event.occurred_at(),
    ))
}

fn friendship_record(event: &FriendshipDomainEvent) -> Result<OutboxRecord, RepositoryError> {
    let payload = match event {
        FriendshipDomainEvent::RequestSent(e) => encode(&FriendRequestSentV1 {
            sender_id: e.sender_id.uuid(),
            receiver_id: e.receiver_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
        FriendshipDomainEvent::RequestAccepted(e) => encode(&FriendRequestAcceptedV1 {
            accepter_id: e.accepter_id.uuid(),
            sender_id: e.sender_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
        FriendshipDomainEvent::Removed(e) => encode(&FriendshipRemovedV1 {
            user_id: e.user_id.uuid(),
            other_id: e.other_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
        FriendshipDomainEvent::Blocked(e) => encode(&UserBlockedV1 {
            blocker_id: e.blocker_id.uuid(),
            blocked_id: e.blocked_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
        FriendshipDomainEvent::Unblocked(e) => encode(&UserUnblockedV1 {
            blocker_id: e.blocker_id.uuid(),
            blocked_id: e.blocked_id.uuid(),
            occurred_at: e.occurred_at,
        })?,
    };
    Ok(record(
        event.aggregate_user_id().uuid(),
        AggregateType::User,
        Some((event.parent_user_id().uuid(), AggregateType::User)),
        event.event_type(),
        payload,
        event.occurred_at(),
    ))
}

fn record(
    aggregate_id: Uuid,
    aggregate_type: AggregateType,
    parent: Option<(Uuid, AggregateType)>,
    event_type: &str,
    payload: Value,
    occurred_at: chrono::DateTime<chrono::Utc>,
) -> OutboxRecord {
    let (parent_id, parent_type) = match parent {
        Some((id, kind)) => (Some(id), Some(kind)),
        None => (None, None),
    };
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id,
        aggregate_type,
        parent_id,
        parent_type,
        event_type: event_type.to_string(),
        payload,
        occurred_at,
        request_id: None,
    }
}

fn uuids(ids: &[UserId]) -> Vec<Uuid> {
    ids.iter().map(|u| u.uuid()).collect()
}

fn value<T: Serialize>(v: &T) -> Result<Value, RepositoryError> {
    serde_json::to_value(v).map_err(|e| RepositoryError::InvalidData(e.to_string()))
}

fn snake_case<T: Serialize>(v: &T) -> Result<String, RepositoryError> {
    value(v)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| RepositoryError::InvalidData("expected string enum".to_string()))
}

fn encode<T: Serialize>(v: &T) -> Result<Value, RepositoryError> {
    value(v)
}

fn reply_json(reply: &messaging_domain::message::reply::ReplyPreview) -> Value {
    serde_json::json!({
        "id": reply.message_id,
        "authorId": reply.author_id.uuid(),
        "content": reply.snippet,
        "hasAttachments": reply.has_attachments,
        "voiceDurationMs": reply.voice_duration_ms,
        "videoNoteDurationMs": reply.video_note_duration_ms,
        "videoNotePoster": reply.video_note_poster.as_ref().map(|p| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(p)
        }),
        "hiddenFor": reply.hidden_for.iter().map(|u| u.uuid()).collect::<Vec<_>>(),
    })
}
