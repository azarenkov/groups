use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    envelope::Envelope,
    publisher::{PublishError, Publisher},
    recipients::RecipientResolver,
};

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Recipient lookup error: {0}")]
    Recipients(#[from] sqlx::Error),
    #[error("Publish error: {0}")]
    Publish(#[from] PublishError),
}

pub async fn dispatch(
    subject: &str,
    payload: &[u8],
    resolver: &RecipientResolver,
    publisher: &Publisher,
) -> Result<(), DispatchError> {
    if ends_with(subject, ".dm_message.sent") {
        let evt: DmSent = serde_json::from_slice(payload)?;
        let invite = resolver.dm_invite_for(evt.message_id).await?;
        let call = resolver.dm_call_for(evt.message_id).await?;
        let sender_name = resolver.user_name(evt.sender_id).await?;
        let envelope = Envelope::new(
            "dm_message.sent",
            json!({
                "id": evt.message_id,
                "senderId": evt.sender_id,
                "senderName": sender_name,
                "recipientId": evt.recipient_id,
                "content": evt.content,
                "occurredAt": evt.occurred_at,
                "inviteServerId": invite.as_ref().map(|i| i.0),
                "inviteServerName": invite.as_ref().map(|i| i.1.clone()),
                "callStatus": call.as_ref().map(|c| c.0.clone()),
                "callDurationSeconds": call.as_ref().and_then(|c| c.1),
            }),
        );
        publisher
            .publish_to_users(unique([evt.sender_id, evt.recipient_id]), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".dm_message.edited") {
        let evt: DmEdited = serde_json::from_slice(payload)?;
        let mut recipients: Vec<Uuid> = vec![evt.sender_id];
        if let Some(rid) = resolver.dm_recipient_for(evt.message_id, evt.sender_id).await? {
            recipients.push(rid);
        }
        let envelope = Envelope::new(
            "dm_message.edited",
            json!({
                "id": evt.message_id,
                "senderId": evt.sender_id,
                "newContent": evt.new_content,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".dm_message.deleted") {
        let evt: DmDeleted = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "dm_message.deleted",
            json!({
                "id": evt.message_id,
                "senderId": evt.sender_id,
                "recipientId": evt.recipient_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique([evt.sender_id, evt.recipient_id]), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".dm.read") {
        let evt: DmReadDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "dm.read",
            json!({
                "readerId": evt.reader_id,
                "readAt": evt.read_at,
            }),
        );
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.read") {
        let evt: ChannelReadDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "channel.read",
            json!({
                "channelId": evt.channel_id,
                "readerId": evt.reader_id,
                "readAt": evt.read_at,
            }),
        );
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".message.sent") {
        let evt: ChannelMsgSent = serde_json::from_slice(payload)?;
        let recipients = if !evt.recipient_ids.is_empty() {
            evt.recipient_ids.clone()
        } else {
            resolver.text_channel_recipients(evt.channel_id).await?
        };
        let author_name = resolver.user_name(evt.author_id).await?;
        let envelope = Envelope::new(
            "message.sent",
            json!({
                "id": evt.message_id,
                "channelId": evt.channel_id,
                "authorId": evt.author_id,
                "authorName": author_name,
                "content": evt.content,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".message.edited") {
        let evt: ChannelMsgEdited = serde_json::from_slice(payload)?;
        let recipients = if !evt.recipient_ids.is_empty() {
            evt.recipient_ids.clone()
        } else {
            resolver.text_channel_recipients(evt.channel_id).await?
        };
        let envelope = Envelope::new(
            "message.edited",
            json!({
                "id": evt.message_id,
                "channelId": evt.channel_id,
                "newContent": evt.new_content,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".message.deleted") {
        let evt: ChannelMsgDeleted = serde_json::from_slice(payload)?;
        let recipients = if !evt.recipient_ids.is_empty() {
            evt.recipient_ids.clone()
        } else {
            resolver.text_channel_recipients(evt.channel_id).await?
        };
        let envelope = Envelope::new(
            "message.deleted",
            json!({
                "id": evt.message_id,
                "channelId": evt.channel_id,
                "deletedBy": evt.deleted_by,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.created") {
        let evt: ChannelCreatedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.created",
            json!({
                "id": evt.channel_id,
                "serverId": evt.server_id,
                "ownerId": evt.owner_id,
                "name": evt.name,
                "kind": evt.kind,
                "visibility": evt.visibility,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.renamed") {
        let evt: ChannelRenamedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.renamed",
            json!({
                "id": evt.channel_id,
                "serverId": evt.server_id,
                "oldName": evt.old_name,
                "newName": evt.new_name,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.deleted") {
        let evt: ChannelDeletedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.deleted",
            json!({
                "id": evt.channel_id,
                "serverId": evt.server_id,
                "deletedBy": evt.deleted_by,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.participant.joined") {
        let evt: ParticipantSimple = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let user_name = resolver.user_name(evt.user_id).await?;
        let envelope = Envelope::new(
            "channel.participant.joined",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "userName": user_name,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.participant.left") {
        let evt: ParticipantLeftDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.participant.left",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "reason": evt.reason,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.participant.muted") {
        let evt: ParticipantMutedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.participant.muted",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "mutedBy": evt.muted_by,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.participant.unmuted") {
        let evt: ParticipantUnmutedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.participant.unmuted",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "unmutedBy": evt.unmuted_by,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.stream.viewers") {
        let evt: StreamViewersDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.stream.viewers",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "streamerId": evt.streamer_id,
                "count": evt.count,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.participant.camera.on")
        || ends_with(subject, ".channel.participant.camera.off")
        || ends_with(subject, ".channel.participant.screen.on")
        || ends_with(subject, ".channel.participant.screen.off")
    {
        let evt: ParticipantMediaDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let ws_event = match (evt.source.as_str(), evt.enabled) {
            ("camera", true) => "channel.participant.camera.on",
            ("camera", false) => "channel.participant.camera.off",
            ("screen", true) => "channel.participant.screen.on",
            _ => "channel.participant.screen.off",
        };
        let envelope = Envelope::new(
            ws_event,
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "source": evt.source,
                "enabled": evt.enabled,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.participant.connection_aborted") {
        let evt: ParticipantConnectionAbortedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.participant.connectionAborted",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "reason": evt.reason,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.text.joined") {
        let evt: TextJoinedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.text_channel_recipients(evt.channel_id).await?;
        let envelope = Envelope::new(
            "channel.text.joined",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "joinedAt": evt.joined_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".server.created") {
        let evt: ServerCreatedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "server.created",
            json!({
                "id": evt.server_id,
                "ownerId": evt.owner_id,
                "name": evt.name,
                "description": evt.description,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique([evt.owner_id]), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".server.deleted") {
        let evt: ServerDeletedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "server.deleted",
            json!({
                "id": evt.server_id,
                "ownerId": evt.owner_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        let mut recipients: Vec<Uuid> = evt.member_ids.unwrap_or_default();
        recipients.push(evt.owner_id);
        publisher
            .publish_to_users(unique(recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".call.ring") {
        let evt: CallEventDto = serde_json::from_slice(payload)?;
        let caller_name = resolver.user_name(evt.actor_id).await?;
        let envelope = Envelope::new(
            "call.incoming",
            json!({
                "callId": evt.call_id,
                "callerId": evt.actor_id,
                "callerName": caller_name,
            }),
        );
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".call.participant.joined") {
        let evt: CallEventDto = serde_json::from_slice(payload)?;
        let name = resolver.user_name(evt.actor_id).await?;
        let envelope = Envelope::new(
            "call.participant.joined",
            json!({
                "callId": evt.call_id,
                "userId": evt.actor_id,
                "userName": name,
            }),
        );
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".call.declined") {
        let evt: CallEventDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "call.declined",
            json!({
                "callId": evt.call_id,
                "userId": evt.actor_id,
            }),
        );
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".call.available") {
        let evt: CallEventDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "call.available",
            json!({
                "callId": evt.call_id,
                "userId": evt.actor_id,
            }),
        );
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".call.ended") {
        let evt: CallEventDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "call.ended",
            json!({
                "callId": evt.call_id,
            }),
        );
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".call.participant.left") {
        let evt: CallEventDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "call.participant.left",
            json!({
                "callId": evt.call_id,
                "userId": evt.actor_id,
            }),
        );
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".user.request_sent") {
        let evt: FriendRequestSentDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "friendship.requestSent",
            json!({
                "senderId": evt.sender_id,
                "receiverId": evt.receiver_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique([evt.sender_id, evt.receiver_id]), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".user.request_accepted") {
        let evt: FriendRequestAcceptedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "friendship.requestAccepted",
            json!({
                "accepterId": evt.accepter_id,
                "senderId": evt.sender_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique([evt.accepter_id, evt.sender_id]), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".user.removed") {
        let evt: FriendshipRemovedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "friendship.removed",
            json!({
                "userId": evt.user_id,
                "otherId": evt.other_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique([evt.user_id, evt.other_id]), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".user.registered")
        || ends_with(subject, ".user.email_verified")
        || ends_with(subject, ".user.phone_verified")
        || ends_with(subject, ".user.logged_in")
        || ends_with(subject, ".user.logged_out")
    {
        let evt: UserLifecycleDto = serde_json::from_slice(payload)?;
        let event_type = subject.rsplit('.').next().unwrap_or("event");
        let envelope = Envelope::new(
            match event_type {
                "registered" => "user.registered",
                "email_verified" => "user.emailVerified",
                "phone_verified" => "user.phoneVerified",
                "logged_in" => "user.loggedIn",
                "logged_out" => "user.loggedOut",
                _ => "user.event",
            },
            json!({
                "userId": evt.user_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique([evt.user_id]), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".user.verification.issued") {
        tracing::debug!(%subject, "Skipping verification event (contains secret code)");
        return Ok(());
    }

    tracing::debug!(%subject, "No realtime handler for subject; dropping");
    Ok(())
}

fn ends_with(subject: &str, suffix: &str) -> bool {
    subject == &suffix[1..] || subject.ends_with(suffix)
}

fn unique<I: IntoIterator<Item = Uuid>>(iter: I) -> Vec<Uuid> {
    let mut seen = HashSet::new();
    iter.into_iter().filter(|u| seen.insert(*u)).collect()
}

#[derive(Deserialize)]
struct DmSent {
    message_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    content: Value,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct DmEdited {
    message_id: Uuid,
    sender_id: Uuid,
    new_content: Value,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct DmDeleted {
    message_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct DmReadDto {
    reader_id: Uuid,
    read_at: DateTime<Utc>,
    recipients: Vec<Uuid>,
}

#[derive(Deserialize)]
struct ChannelReadDto {
    channel_id: Uuid,
    reader_id: Uuid,
    read_at: DateTime<Utc>,
    recipients: Vec<Uuid>,
}

#[derive(Deserialize)]
struct ChannelMsgSent {
    message_id: Uuid,
    channel_id: Uuid,
    author_id: Uuid,
    content: Value,
    occurred_at: DateTime<Utc>,
    #[serde(default)]
    recipient_ids: Vec<Uuid>,
}

#[derive(Deserialize)]
struct ChannelMsgEdited {
    message_id: Uuid,
    channel_id: Uuid,
    new_content: Value,
    occurred_at: DateTime<Utc>,
    #[serde(default)]
    recipient_ids: Vec<Uuid>,
}

#[derive(Deserialize)]
struct ChannelMsgDeleted {
    message_id: Uuid,
    channel_id: Uuid,
    deleted_by: Uuid,
    occurred_at: DateTime<Utc>,
    #[serde(default)]
    recipient_ids: Vec<Uuid>,
}

#[derive(Deserialize)]
struct ChannelCreatedDto {
    channel_id: Uuid,
    server_id: Uuid,
    owner_id: Uuid,
    name: Value,
    kind: Value,
    visibility: Value,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ChannelRenamedDto {
    channel_id: Uuid,
    server_id: Uuid,
    old_name: Value,
    new_name: Value,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ChannelDeletedDto {
    channel_id: Uuid,
    server_id: Uuid,
    deleted_by: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ParticipantSimple {
    channel_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ParticipantLeftDto {
    channel_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    reason: Value,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ParticipantMutedDto {
    channel_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    muted_by: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ParticipantUnmutedDto {
    channel_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    unmuted_by: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ParticipantMediaDto {
    channel_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    source: String,
    enabled: bool,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct StreamViewersDto {
    channel_id: Uuid,
    server_id: Uuid,
    streamer_id: Uuid,
    count: i64,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ParticipantConnectionAbortedDto {
    channel_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    reason: Value,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct TextJoinedDto {
    channel_id: Uuid,
    server_id: Uuid,
    user_id: Uuid,
    joined_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ServerCreatedDto {
    server_id: Uuid,
    owner_id: Uuid,
    name: Value,
    description: Option<Value>,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ServerDeletedDto {
    server_id: Uuid,
    owner_id: Uuid,
    #[serde(default)]
    member_ids: Option<Vec<Uuid>>,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct CallEventDto {
    call_id: Uuid,
    actor_id: Uuid,
    recipients: Vec<Uuid>,
}

#[derive(Deserialize)]
struct FriendRequestSentDto {
    sender_id: Uuid,
    receiver_id: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct FriendRequestAcceptedDto {
    accepter_id: Uuid,
    sender_id: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct FriendshipRemovedDto {
    user_id: Uuid,
    other_id: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct UserLifecycleDto {
    user_id: Uuid,
    occurred_at: DateTime<Utc>,
}
