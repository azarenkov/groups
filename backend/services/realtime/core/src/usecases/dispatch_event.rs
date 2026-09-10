use contracts::events::{
    call::CallEventV1 as CallEventDto,
    channel::{
        ChannelCreatedV1 as ChannelCreatedDto, ChannelDeletedV1 as ChannelDeletedDto,
        ChannelRenamedV1 as ChannelRenamedDto,
        ParticipantConnectionAbortedV1 as ParticipantConnectionAbortedDto,
        ParticipantJoinedV1 as ParticipantSimple, ParticipantLeftV1 as ParticipantLeftDto,
        ParticipantMediaV1 as ParticipantMediaDto, ParticipantMutedV1 as ParticipantMutedDto,
        ParticipantUnmutedV1 as ParticipantUnmutedDto, ParticipantDeafenedV1 as ParticipantDeafenedDto, ParticipantUndeafenedV1 as ParticipantUndeafenedDto, StreamViewersV1 as StreamViewersDto,
        TextChannelJoinedV1 as TextJoinedDto,
    },
    dm_message::{
        DmMessageDeletedV1 as DmDeleted, DmMessageEditedV1 as DmEdited,
        DmMessageSentV1 as DmSent, DmReadV1 as DmReadDto,
    },
    friendship::{
        FriendRequestAcceptedV1 as FriendRequestAcceptedDto,
        FriendRequestSentV1 as FriendRequestSentDto, FriendshipRemovedV1 as FriendshipRemovedDto,
    },
    message::{
        ChannelMessageDeletedV1 as ChannelMsgDeleted, ChannelMessageEditedV1 as ChannelMsgEdited,
        ChannelMessageSentV1 as ChannelMsgSent, ChannelReadV1 as ChannelReadDto,
    },
    server::{
        ServerCreatedV1 as ServerCreatedDto, ServerDeletedV1 as ServerDeletedDto,
        ServerMemberV1 as ServerMemberDto, ServerRolesChangedV1 as ServerRolesChangedDto,
        ServerOwnerChangedV1 as ServerOwnerChangedDto, ServerUpdatedV1 as ServerUpdatedDto,
    },
    user::{
        NotificationPreferenceUpdatedV1 as NotificationPrefUpdated,
        UserActivityChangedV1 as UserActivityChangedDto, UserLifecycleV1 as UserLifecycleDto,
        UserProfileChangedV1 as UserProfileChangedDto,
        UserRenamedV1 as UserRenamedDto,
    },
};
use std::collections::HashSet;

use serde_json::json;
use uuid::Uuid;

use ports::repository::RepositoryError;
use realtime_domain::envelope::Envelope;

use crate::ports::{directory::RecipientDirectory, projection::RealtimeProjection, publisher::{PublishError, RealtimePublisher}};

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Recipient lookup error: {0}")]
    Recipients(#[from] RepositoryError),
    #[error("Publish error: {0}")]
    Publish(#[from] PublishError),
}

pub struct DispatchEvent<D: RecipientDirectory + RealtimeProjection, P: RealtimePublisher> {
    directory: std::sync::Arc<D>,
    publisher: std::sync::Arc<P>,
}

impl<D: RecipientDirectory + RealtimeProjection, P: RealtimePublisher> DispatchEvent<D, P> {
    pub fn new(directory: std::sync::Arc<D>, publisher: std::sync::Arc<P>) -> Self {
        Self { directory, publisher }
    }

    pub async fn execute(&self, subject: &str, payload: &[u8]) -> Result<(), DispatchError> {
    let resolver = self.directory.as_ref();
    let publisher = self.publisher.as_ref();
    if ends_with(subject, ".dm_message.sent") {
        let evt: DmSent = serde_json::from_slice(payload)?;
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
                "inviteServerId": evt.invite_server_id,
                "inviteServerName": evt.invite_server_name,
                "callStatus": evt.call_status,
                "callDurationSeconds": evt.call_duration_seconds,
                "attachments": evt.attachments.unwrap_or_else(|| json!([])),
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
        if let Some(rid) = evt.recipient_id {
            recipients.push(rid);
        }
        let envelope = Envelope::new(
            "dm_message.edited",
            json!({
                "id": evt.message_id,
                "senderId": evt.sender_id,
                "newContent": evt.new_content,
                "occurredAt": evt.occurred_at,
                "attachments": evt.attachments.unwrap_or_else(|| json!([])),
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
        let mut data = json!({
            "readerId": evt.reader_id,
            "readAt": evt.read_at,
        });
        if let Some(peer_id) = evt.peer_id {
            data["peerId"] = json!(peer_id);
        }
        let envelope = Envelope::new("dm.read", data);
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".notification_preference.updated") {
        let evt: NotificationPrefUpdated = serde_json::from_slice(payload)?;
        let mut data = json!({
            "userId": evt.user_id,
            "scopeType": evt.scope_type,
            "scopeId": evt.scope_id,
            "level": evt.level,
        });
        if let Some(version) = evt.version {
            data["version"] = json!(version);
        }
        let envelope = Envelope::new("notification_preference.updated", data);
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".server.roles.changed") {
        let evt: ServerRolesChangedDto = serde_json::from_slice(payload)?;
        let mut recipients = resolver.server_members(evt.server_id).await?;
        recipients.extend(evt.recipients);
        let envelope = Envelope::new(
            "server.roles.changed",
            json!({ "serverId": evt.server_id }),
        );
        publisher
            .publish_to_users(unique(recipients), &envelope)
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
        let recipients = evt.recipient_ids.clone();
        let author_name = resolver.user_name(evt.author_id).await?;
        let attachments = evt.attachments.clone().unwrap_or_else(|| json!([]));
        let mut data = json!({
            "id": evt.message_id,
            "channelId": evt.channel_id,
            "authorId": evt.author_id,
            "authorName": author_name,
            "content": evt.content,
            "occurredAt": evt.occurred_at,
            "attachments": attachments,
        });
        if let Some(mentions) = &evt.mentions {
            data["mentions"] = json!(mentions);
        }
        data["kind"] = json!(if evt.system.is_some() { "system" } else { "user" });
        if let Some(system) = &evt.system {
            data["system"] = system.clone();
        }
        let envelope = Envelope::new("message.sent", data);
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".message.edited") {
        let evt: ChannelMsgEdited = serde_json::from_slice(payload)?;
        let recipients = evt.recipient_ids.clone();
        let attachments = evt.attachments.clone().unwrap_or_else(|| json!([]));
        let envelope = Envelope::new(
            "message.edited",
            json!({
                "id": evt.message_id,
                "channelId": evt.channel_id,
                "newContent": evt.new_content,
                "occurredAt": evt.occurred_at,
                "attachments": attachments,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".message.deleted") {
        let evt: ChannelMsgDeleted = serde_json::from_slice(payload)?;
        let recipients = evt.recipient_ids.clone();
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
        let recipients = if evt.recipient_ids.is_empty() {
            resolver.server_members(evt.server_id).await?
        } else {
            evt.recipient_ids.clone()
        };
        let envelope = Envelope::new(
            "channel.created",
            json!({
                "id": evt.channel_id,
                "serverId": evt.server_id,
                "ownerId": evt.owner_id,
                "isDefault": evt.is_default,
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
        let recipients = if evt.recipient_ids.is_empty() {
            resolver.server_members(evt.server_id).await?
        } else {
            evt.recipient_ids.clone()
        };
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

    if ends_with(subject, ".channel.participant.deafened") {
        let evt: ParticipantDeafenedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.participant.deafened",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "deafenedBy": evt.deafened_by,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.participant.undeafened") {
        let evt: ParticipantUndeafenedDto = serde_json::from_slice(payload)?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let envelope = Envelope::new(
            "channel.participant.undeafened",
            json!({
                "channelId": evt.channel_id,
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "undeafenedBy": evt.undeafened_by,
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
        let recipients = evt.recipient_ids.clone();
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
        resolver.add_server_member(evt.server_id, evt.owner_id, evt.occurred_at).await?;
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

    if ends_with(subject, ".server.owner_changed") {
        let evt: ServerOwnerChangedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "server.ownerChanged",
            json!({
                "id": evt.server_id,
                "ownerId": evt.owner_id,
                "previousOwnerId": evt.previous_owner_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique(evt.member_ids), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".server.updated") {
        let evt: ServerUpdatedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "server.updated",
            json!({
                "id": evt.server_id,
                "name": evt.name,
                "description": evt.description,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique(evt.member_ids), &envelope)
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
        resolver.remove_server(evt.server_id, evt.occurred_at).await?;
        return Ok(());
    }

    if ends_with(subject, ".server.member_joined") {
        let evt: ServerMemberDto = serde_json::from_slice(payload)?;
        resolver.add_server_member(evt.server_id, evt.user_id, evt.occurred_at).await?;
        let recipients = resolver.server_members(evt.server_id).await?;
        let user_name = resolver.user_name_versioned(evt.user_id).await?;
        let envelope = Envelope::new(
            "server.memberJoined",
            json!({
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "userName": user_name.as_ref().map(|(name, _)| name.clone()),
                "userNameUpdatedAt": user_name.as_ref().map(|(_, at)| *at),
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".server.member_left") {
        let evt: ServerMemberDto = serde_json::from_slice(payload)?;
        let mut recipients = resolver.server_members(evt.server_id).await?;
        recipients.push(evt.user_id);
        resolver.remove_server_member(evt.server_id, evt.user_id, evt.occurred_at).await?;
        let envelope = Envelope::new(
            "server.memberLeft",
            json!({
                "serverId": evt.server_id,
                "userId": evt.user_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
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
                "video": evt.video,
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
        resolver
            .add_pending_friend_edge(evt.sender_id, evt.receiver_id, evt.occurred_at)
            .await?;
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
        resolver.add_friend_edge(evt.accepter_id, evt.sender_id, evt.occurred_at).await?;
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
        resolver.remove_friend_edge(evt.user_id, evt.other_id, evt.occurred_at).await?;
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

    if ends_with(subject, ".user.profile_changed") {
        let evt: UserProfileChangedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "user.profileChanged",
            json!({
                "userId": evt.user_id,
                "bio": evt.bio,
                "showActivity": evt.show_activity,
                "showGameActivity": evt.show_game_activity,
                "occurredAt": evt.occurred_at,
            }),
        );
        let mut recipients = resolver.rename_recipients(evt.user_id).await?;
        recipients.push(evt.user_id);
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".user.activity_changed") {
        let evt: UserActivityChangedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "user.activityChanged",
            json!({
                "userId": evt.user_id,
                "gameId": evt.game_id,
                "name": evt.name,
                "startedAt": evt.started_at,
                "occurredAt": evt.occurred_at,
            }),
        );
        let mut recipients = resolver.interested_in(evt.user_id).await?;
        recipients.push(evt.user_id);
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".user.renamed") {
        let evt: UserRenamedDto = serde_json::from_slice(payload)?;
        resolver.upsert_user_name(evt.user_id, &evt.name, evt.occurred_at).await?;
        let envelope = Envelope::new(
            "user.renamed",
            json!({
                "userId": evt.user_id,
                "name": evt.name,
                "occurredAt": evt.occurred_at,
            }),
        );
        let mut recipients = resolver.rename_recipients(evt.user_id).await?;
        recipients.push(evt.user_id);
        publisher.publish_to_users(unique(recipients), &envelope).await?;
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
        if event_type == "registered"
            && let Some(name) = &evt.name
        {
            resolver
                .upsert_user_name(evt.user_id, name, evt.occurred_at)
                .await?;
        }
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
}

fn ends_with(subject: &str, suffix: &str) -> bool {
    subject == &suffix[1..] || subject.ends_with(suffix)
}

fn unique<I: IntoIterator<Item = Uuid>>(iter: I) -> Vec<Uuid> {
    let mut seen = HashSet::new();
    iter.into_iter().filter(|u| seen.insert(*u)).collect()
}
