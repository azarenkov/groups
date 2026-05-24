use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    connections::{PublishError, Publisher},
    envelope::Envelope,
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
        let envelope = Envelope::new(
            "dm_message.sent",
            json!({
                "id": evt.message_id,
                "senderId": evt.sender_id,
                "recipientId": evt.recipient_id,
                "content": evt.content,
                "occurredAt": evt.occurred_at,
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

    if ends_with(subject, ".message.sent") {
        let evt: ChannelMsgSent = serde_json::from_slice(payload)?;
        let recipients = resolver.channel_members(evt.channel_id).await?;
        let envelope = Envelope::new(
            "message.sent",
            json!({
                "id": evt.message_id,
                "channelId": evt.channel_id,
                "authorId": evt.author_id,
                "content": evt.content,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".message.edited") {
        let evt: ChannelMsgEdited = serde_json::from_slice(payload)?;
        let recipients = resolver.channel_members(evt.channel_id).await?;
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
        let recipients = resolver.channel_members(evt.channel_id).await?;
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
        let envelope = Envelope::new(
            "channel.participant.joined",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
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
        let recipients = resolver.server_members(evt.server_id).await?;
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
        publisher
            .publish_to_users(unique([evt.owner_id]), &envelope)
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
struct ChannelMsgSent {
    message_id: Uuid,
    channel_id: Uuid,
    author_id: Uuid,
    content: Value,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ChannelMsgEdited {
    message_id: Uuid,
    channel_id: Uuid,
    new_content: Value,
    occurred_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ChannelMsgDeleted {
    message_id: Uuid,
    channel_id: Uuid,
    deleted_by: Uuid,
    occurred_at: DateTime<Utc>,
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
    occurred_at: DateTime<Utc>,
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
