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
        DmConversationClearedV1 as DmClearedDto, DmConversationHiddenChangedV1 as DmHiddenDto,
    },
    friendship::{
        FriendRequestAcceptedV1 as FriendRequestAcceptedDto,
        FriendRequestSentV1 as FriendRequestSentDto, FriendshipRemovedV1 as FriendshipRemovedDto, UserBlockedV1 as UserBlockedDto, UserUnblockedV1 as UserUnblockedDto,
    },
    message::{
        ChannelMessageDeletedV1 as ChannelMsgDeleted, ChannelMessageEditedV1 as ChannelMsgEdited,
        ChannelMessageSentV1 as ChannelMsgSent, ChannelReadV1 as ChannelReadDto, ConversationPinsUpdatedV1 as ConversationPinsUpdatedDto,
        MessageHiddenV1 as MessageHiddenDto, MessagePinChangedV1 as MessagePinChangedDto, VoiceListenedV1 as VoiceListenedDto,
    },
    community::{
        CommunityCreatedV1 as CommunityCreatedDto, CommunityDeletedV1 as CommunityDeletedDto,
        CommunityMemberV1 as CommunityMemberDto, CommunityRolesChangedV1 as CommunityRolesChangedDto,
        CommunityOwnerChangedV1 as CommunityOwnerChangedDto, CommunityUpdatedV1 as CommunityUpdatedDto,
    },
    user::{
        NotificationPreferenceUpdatedV1 as NotificationPrefUpdated,
        UserActivityChangedV1 as UserActivityChangedDto, UserLifecycleV1 as UserLifecycleDto,
        UserEntitlementsChangedV1 as UserEntitlementsChangedDto,
        UserTrialEndingV1 as UserTrialEndingDto,
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
    #[error("Block lookup error: {0}")]
    Blocks(#[from] crate::ports::blocks::RelationLookupError),
}

fn split_hidden_reply(reply: Option<&serde_json::Value>) -> (Option<serde_json::Value>, Vec<Uuid>) {
    let Some(reply) = reply else {
        return (None, Vec::new());
    };
    let mut reply = reply.clone();
    let hidden_for = reply
        .as_object_mut()
        .and_then(|o| o.remove("hiddenFor"))
        .and_then(|v| serde_json::from_value::<Vec<Uuid>>(v).ok())
        .unwrap_or_default();
    (Some(reply), hidden_for)
}

fn deleted_reply(reply: &serde_json::Value) -> serde_json::Value {
    json!({
        "id": reply.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "content": "",
        "hasAttachments": false,
        "deleted": true,
    })
}

async fn publish_with_reply<P: RealtimePublisher>(
    publisher: &P,
    kind: &'static str,
    mut data: serde_json::Value,
    reply: Option<serde_json::Value>,
    hidden_for: &[Uuid],
    recipients: Vec<Uuid>,
) -> Result<(), DispatchError> {
    let Some(reply) = reply else {
        publisher.publish_to_users(recipients, &Envelope::new(kind, data)).await?;
        return Ok(());
    };
    let (hiding, seeing): (Vec<Uuid>, Vec<Uuid>) =
        recipients.into_iter().partition(|r| hidden_for.contains(r));
    if !hiding.is_empty() {
        let mut masked = data.clone();
        masked["replyTo"] = deleted_reply(&reply);
        publisher.publish_to_users(hiding, &Envelope::new(kind, masked)).await?;
    }
    if !seeing.is_empty() {
        data["replyTo"] = reply;
        publisher.publish_to_users(seeing, &Envelope::new(kind, data)).await?;
    }
    Ok(())
}

async fn named_reply<D: RecipientDirectory>(
    resolver: &D,
    reply: Option<&serde_json::Value>,
) -> Result<Option<serde_json::Value>, DispatchError> {
    let Some(reply) = reply else {
        return Ok(None);
    };
    let mut reply = reply.clone();
    let author = reply
        .get("authorId")
        .and_then(|v| v.as_str())
        .and_then(|v| Uuid::parse_str(v).ok());
    if let Some(author) = author {
        reply["authorName"] = json!(resolver.user_name(author).await?);
    }
    Ok(Some(reply))
}

async fn confirm_pending_dm_peers<D: RecipientDirectory + RealtimeProjection, B: crate::ports::blocks::RelationDirectory>(
    resolver: &D,
    relations: &B,
    user_id: Uuid,
    recipients: &mut Vec<Uuid>,
) -> Result<(), DispatchError> {
    let pending = resolver.pending_dm_peers(user_id).await?;
    if pending.is_empty() {
        return Ok(());
    }
    let confirmed = relations.dm_peers(user_id, &pending).await?;
    for peer in &confirmed {
        resolver.record_dm_pair(user_id, *peer).await?;
    }
    recipients.extend(confirmed);
    Ok(())
}

async fn forget_pair_without_history<D: RecipientDirectory + RealtimeProjection, B: crate::ports::blocks::RelationDirectory>(
    resolver: &D,
    relations: &B,
    a: Uuid,
    b: Uuid,
) -> Result<(), DispatchError> {
    if relations.dm_peers(a, &[b]).await?.is_empty() {
        resolver.remove_dm_pair(a, b).await?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSequence {
    pub partition: String,
    pub seq: i64,
    pub members: Vec<Uuid>,
}

struct Sequenced<'a, P: RealtimePublisher> {
    inner: &'a P,
    partition: String,
    seq: i64,
    reached: std::sync::Mutex<std::collections::HashSet<Uuid>>,
}

#[async_trait::async_trait]
impl<P: RealtimePublisher> RealtimePublisher for Sequenced<'_, P> {
    async fn publish_to_users(&self, recipients: Vec<Uuid>, envelope: &Envelope<'_>) -> Result<(), PublishError> {
        self.reached
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(recipients.iter().copied());
        self.inner
            .publish_to_users(recipients, &envelope.sequenced(&self.partition, self.seq))
            .await
    }
}

struct Unpublished;

#[async_trait::async_trait]
impl RealtimePublisher for Unpublished {
    async fn publish_to_users(&self, _: Vec<Uuid>, _: &Envelope<'_>) -> Result<(), PublishError> {
        Ok(())
    }
}

pub struct DispatchEvent<D: RecipientDirectory + RealtimeProjection, P: RealtimePublisher, B: crate::ports::blocks::RelationDirectory> {
    directory: std::sync::Arc<D>,
    publisher: std::sync::Arc<P>,
    blocks: std::sync::Arc<B>,
}

impl<D: RecipientDirectory + RealtimeProjection, P: RealtimePublisher, B: crate::ports::blocks::RelationDirectory> DispatchEvent<D, P, B> {
    pub fn new(directory: std::sync::Arc<D>, publisher: std::sync::Arc<P>, blocks: std::sync::Arc<B>) -> Self {
        Self { directory, publisher, blocks }
    }

    pub async fn execute(&self, subject: &str, payload: &[u8]) -> Result<(), DispatchError> {
        self.run(subject, payload, self.publisher.as_ref()).await
    }

    pub async fn execute_unpublished(&self, subject: &str, payload: &[u8]) -> Result<(), DispatchError> {
        self.run(subject, payload, &Unpublished).await
    }

    pub async fn execute_sequenced(
        &self,
        subject: &str,
        payload: &[u8],
        sequence: Option<EventSequence>,
    ) -> Result<(), DispatchError> {
        match sequence {
            Some(sequence) => {
                let stamped = Sequenced {
                    inner: self.publisher.as_ref(),
                    partition: sequence.partition,
                    seq: sequence.seq,
                    reached: std::sync::Mutex::new(std::collections::HashSet::new()),
                };
                self.run(subject, payload, &stamped).await?;
                let reached = stamped.reached.into_inner().unwrap_or_else(|e| e.into_inner());
                let missing: Vec<Uuid> = unique(sequence.members.into_iter().filter(|m| !reached.contains(m)));
                if !missing.is_empty() {
                    self.publisher
                        .publish_to_users(
                            missing,
                            &Envelope::new("sequence.skip", json!({})).sequenced(&stamped.partition, stamped.seq),
                        )
                        .await?;
                }
                Ok(())
            }
            None => self.execute(subject, payload).await,
        }
    }

    async fn run<Q: RealtimePublisher>(&self, subject: &str, payload: &[u8], publisher: &Q) -> Result<(), DispatchError> {
    let resolver = self.directory.as_ref();
    let blocks = self.blocks.as_ref();
    if ends_with(subject, ".dm_message.sent") {
        let evt: DmSent = serde_json::from_slice(payload)?;
        resolver.record_dm_pair(evt.sender_id, evt.recipient_id).await?;
        let sender_name = resolver.user_name(evt.sender_id).await?;
        let (reply_to, hidden_for) = split_hidden_reply(evt.reply_to.as_ref());
        let reply_to = named_reply(resolver, reply_to.as_ref()).await?;
        let data = json!({
            "id": evt.message_id,
            "senderId": evt.sender_id,
            "senderName": sender_name,
            "recipientId": evt.recipient_id,
            "content": evt.content,
            "occurredAt": evt.occurred_at,
            "inviteCommunityId": evt.invite_community_id,
            "inviteCommunityName": evt.invite_community_name,
            "callStatus": evt.call_status,
            "callDurationSeconds": evt.call_duration_seconds,
            "attachments": evt.attachments.unwrap_or_else(|| json!([])),
            "forwardedFrom": evt.forwarded_from,
            "sticker": evt.sticker,
        });
        publish_with_reply(
            publisher,
            "dm_message.sent",
            data,
            reply_to,
            &hidden_for,
            unique([evt.sender_id, evt.recipient_id]),
        )
        .await?;
        return Ok(());
    }

    if ends_with(subject, ".dm_message.edited") {
        let evt: DmEdited = serde_json::from_slice(payload)?;
        let mut recipients: Vec<Uuid> = vec![evt.sender_id];
        if let Some(rid) = evt.recipient_id {
            recipients.push(rid);
        }
        recipients.retain(|r| !evt.hidden_for.contains(r));
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
        forget_pair_without_history(resolver, blocks, evt.sender_id, evt.recipient_id).await?;
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

    if ends_with(subject, ".community.roles.changed") {
        let evt: CommunityRolesChangedDto = serde_json::from_slice(payload)?;
        let mut recipients = resolver.community_members(evt.community_id).await?;
        recipients.extend(evt.recipients);
        let envelope = Envelope::new(
            "community.roles.changed",
            json!({ "communityId": evt.community_id }),
        );
        publisher
            .publish_to_users(unique(recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".voice.listened") {
        let evt: VoiceListenedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "voice.listened",
            json!({
                "messageId": evt.message_id,
                "listenerId": evt.listener_id,
                "listenedAt": evt.listened_at,
            }),
        );
        let skipped: Vec<Uuid> = unique(evt.skipped_ids.into_iter().filter(|id| !evt.recipients.contains(id)));
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        if !skipped.is_empty() {
            publisher.publish_to_users(skipped, &Envelope::new("sequence.skip", json!({}))).await?;
        }
        return Ok(());
    }

    if ends_with(subject, ".message.pin_changed") {
        let evt: MessagePinChangedDto = serde_json::from_slice(payload)?;
        let mut data = json!({
            "messageId": evt.message_id,
            "pinned": evt.pinned,
            "actorId": evt.actor_id,
            "at": evt.occurred_at,
        });
        if let Some(channel_id) = evt.channel_id {
            data["channelId"] = json!(channel_id);
        } else {
            data["participants"] = json!(evt.participants);
        }
        let envelope = Envelope::new("message.pin_changed", data);
        publisher
            .publish_to_users(unique(evt.recipients), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".conversation_pins.updated") {
        let evt: ConversationPinsUpdatedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "conversation_pins.updated",
            json!({
                "dms": evt.dms,
                "communities": evt.communities,
            }),
        );
        publisher.publish_to_users(vec![evt.user_id], &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".dm_conversation.cleared") {
        let evt: DmClearedDto = serde_json::from_slice(payload)?;
        let (recipients, data) = dm_cleared_envelope(&evt);
        publisher
            .publish_to_users(recipients, &Envelope::new("dm_conversation.cleared", data))
            .await?;
        forget_pair_without_history(resolver, blocks, evt.sender_id, evt.recipient_id).await?;
        return Ok(());
    }

    if ends_with(subject, ".dm_conversation.hidden_changed") {
        let evt: DmHiddenDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "dm_conversation.hidden_changed",
            json!({
                "peerId": evt.peer_id,
                "hidden": evt.hidden,
                "at": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(vec![evt.user_id], &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".message.hidden") {
        let evt: MessageHiddenDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "message.hidden",
            json!({
                "messageId": evt.message_id,
                "hiddenAt": evt.hidden_at,
            }),
        );
        publisher.publish_to_users(vec![evt.user_id], &envelope).await?;
        let skipped: Vec<Uuid> = unique(evt.skipped_ids.into_iter().filter(|id| *id != evt.user_id));
        if !skipped.is_empty() {
            publisher.publish_to_users(skipped, &Envelope::new("sequence.skip", json!({}))).await?;
        }
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
        let (reply_to, hidden_for) = split_hidden_reply(evt.reply_to.as_ref());
        let reply_to = named_reply(resolver, reply_to.as_ref()).await?;
        if let Some(forwarded) = &evt.forwarded_from {
            data["forwardedFrom"] = forwarded.clone();
        }
        if let Some(sticker) = &evt.sticker {
            data["sticker"] = sticker.clone();
        }
        publish_with_reply(publisher, "message.sent", data, reply_to, &hidden_for, unique(recipients))
            .await?;
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
        if !evt.skipped_ids.is_empty() {
            publisher
                .publish_to_users(unique(evt.skipped_ids), &Envelope::new("sequence.skip", json!({})))
                .await?;
        }
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
            resolver.community_members(evt.community_id).await?
        } else {
            evt.recipient_ids.clone()
        };
        let envelope = Envelope::new(
            "channel.created",
            json!({
                "id": evt.channel_id,
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
        let envelope = Envelope::new(
            "channel.renamed",
            json!({
                "id": evt.channel_id,
                "communityId": evt.community_id,
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
            resolver.community_members(evt.community_id).await?
        } else {
            evt.recipient_ids.clone()
        };
        let envelope = Envelope::new(
            "channel.deleted",
            json!({
                "id": evt.channel_id,
                "communityId": evt.community_id,
                "deletedBy": evt.deleted_by,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".channel.participant.joined") {
        let evt: ParticipantSimple = serde_json::from_slice(payload)?;
        let recipients = resolver.community_members(evt.community_id).await?;
        let user_name = resolver.user_name(evt.user_id).await?;
        let envelope = Envelope::new(
            "channel.participant.joined",
            json!({
                "channelId": evt.channel_id,
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
        let envelope = Envelope::new(
            "channel.participant.left",
            json!({
                "channelId": evt.channel_id,
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
        let envelope = Envelope::new(
            "channel.participant.muted",
            json!({
                "channelId": evt.channel_id,
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
        let envelope = Envelope::new(
            "channel.participant.unmuted",
            json!({
                "channelId": evt.channel_id,
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
        let envelope = Envelope::new(
            "channel.participant.deafened",
            json!({
                "channelId": evt.channel_id,
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
        let envelope = Envelope::new(
            "channel.participant.undeafened",
            json!({
                "channelId": evt.channel_id,
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
        let envelope = Envelope::new(
            "channel.stream.viewers",
            json!({
                "channelId": evt.channel_id,
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
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
                "communityId": evt.community_id,
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
        let recipients = resolver.community_members(evt.community_id).await?;
        let envelope = Envelope::new(
            "channel.participant.connectionAborted",
            json!({
                "channelId": evt.channel_id,
                "communityId": evt.community_id,
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
                "communityId": evt.community_id,
                "userId": evt.user_id,
                "joinedAt": evt.joined_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".community.created") {
        let evt: CommunityCreatedDto = serde_json::from_slice(payload)?;
        resolver.add_community_member(evt.community_id, evt.owner_id, evt.occurred_at).await?;
        let envelope = Envelope::new(
            "community.created",
            json!({
                "id": evt.community_id,
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

    if ends_with(subject, ".community.owner_changed") {
        let evt: CommunityOwnerChangedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "community.ownerChanged",
            json!({
                "id": evt.community_id,
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

    if ends_with(subject, ".community.updated") {
        let evt: CommunityUpdatedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "community.updated",
            json!({
                "id": evt.community_id,
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

    if ends_with(subject, ".community.deleted") {
        let evt: CommunityDeletedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "community.deleted",
            json!({
                "id": evt.community_id,
                "ownerId": evt.owner_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        let mut recipients: Vec<Uuid> = evt.member_ids.unwrap_or_default();
        recipients.push(evt.owner_id);
        publisher
            .publish_to_users(unique(recipients), &envelope)
            .await?;
        resolver.remove_community(evt.community_id, evt.occurred_at).await?;
        return Ok(());
    }

    if ends_with(subject, ".community.member_joined") {
        let evt: CommunityMemberDto = serde_json::from_slice(payload)?;
        resolver.add_community_member(evt.community_id, evt.user_id, evt.occurred_at).await?;
        let recipients = resolver.community_members(evt.community_id).await?;
        let user_name = resolver.user_name_versioned(evt.user_id).await?;
        let envelope = Envelope::new(
            "community.memberJoined",
            json!({
                "communityId": evt.community_id,
                "userId": evt.user_id,
                "userName": user_name.as_ref().map(|(name, _)| name.clone()),
                "userNameUpdatedAt": user_name.as_ref().map(|(_, at)| *at),
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".community.member_left") {
        let evt: CommunityMemberDto = serde_json::from_slice(payload)?;
        let mut recipients = resolver.community_members(evt.community_id).await?;
        recipients.push(evt.user_id);
        resolver.remove_community_member(evt.community_id, evt.user_id, evt.occurred_at).await?;
        let envelope = Envelope::new(
            "community.memberLeft",
            json!({
                "communityId": evt.community_id,
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

    if ends_with(subject, ".user.blocked") {
        let evt: UserBlockedDto = serde_json::from_slice(payload)?;
        if !resolver
            .apply_block(evt.blocker_id, evt.blocked_id, true, evt.occurred_at)
            .await?
        {
            return Ok(());
        }
        let envelope = Envelope::new(
            "user.blocked",
            json!({
                "blockerId": evt.blocker_id,
                "blockedId": evt.blocked_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique([evt.blocker_id, evt.blocked_id]), &envelope)
            .await?;
        return Ok(());
    }

    if ends_with(subject, ".user.unblocked") {
        let evt: UserUnblockedDto = serde_json::from_slice(payload)?;
        if !resolver
            .apply_block(evt.blocker_id, evt.blocked_id, false, evt.occurred_at)
            .await?
        {
            return Ok(());
        }
        let envelope = Envelope::new(
            "user.unblocked",
            json!({
                "blockerId": evt.blocker_id,
                "blockedId": evt.blocked_id,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher
            .publish_to_users(unique([evt.blocker_id, evt.blocked_id]), &envelope)
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
        let mut payload = json!({
            "userId": evt.user_id,
            "bio": evt.bio,
            "showActivity": evt.show_activity,
            "showGameActivity": evt.show_game_activity,
            "badgeStyle": evt.badge_style,
            "occurredAt": evt.occurred_at,
        });
        if evt.emoji_status_included {
            payload["emojiStatus"] = json!(evt.emoji_status_sticker_id.map(|sticker_id| json!({
                "stickerId": sticker_id,
                "until": evt.emoji_status_until,
            })));
        }
        let envelope = Envelope::new("user.profileChanged", payload);
        let mut recipients = resolver.profile_recipients(evt.user_id).await?;
        confirm_pending_dm_peers(resolver, blocks, evt.user_id, &mut recipients).await?;
        let relations = blocks.block_relations(evt.user_id, &recipients).await?;
        recipients.retain(|r| !relations.blocking.contains(r));
        recipients.push(evt.user_id);
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".user.entitlements_changed") {
        let evt: UserEntitlementsChangedDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "user.entitlementsChanged",
            json!({
                "userId": evt.user_id,
                "tier": evt.tier,
                "validUntil": evt.valid_until,
                "memberNumber": evt.member_number,
                "trial": evt.trial,
                "occurredAt": evt.occurred_at,
            }),
        );
        let mut recipients = resolver.profile_recipients(evt.user_id).await?;
        confirm_pending_dm_peers(resolver, blocks, evt.user_id, &mut recipients).await?;
        let relations = blocks.block_relations(evt.user_id, &recipients).await?;
        recipients.retain(|r| !relations.blocking.contains(r));
        recipients.push(evt.user_id);
        publisher.publish_to_users(unique(recipients), &envelope).await?;
        return Ok(());
    }

    if ends_with(subject, ".user.trial_ending") {
        let evt: UserTrialEndingDto = serde_json::from_slice(payload)?;
        let envelope = Envelope::new(
            "user.trialEnding",
            json!({
                "userId": evt.user_id,
                "endsAt": evt.ends_at,
                "occurredAt": evt.occurred_at,
            }),
        );
        publisher.publish_to_users(vec![evt.user_id], &envelope).await?;
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
        if resolver.appear_offline(evt.user_id).await? {
            publisher.publish_to_users(vec![evt.user_id], &envelope).await?;
            return Ok(());
        }
        let mut recipients = resolver.interested_in(evt.user_id).await?;
        let relations = blocks.block_relations(evt.user_id, &recipients).await?;
        recipients.retain(|r| !relations.blocking.contains(r) && !relations.blocked_by.contains(r));
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
        confirm_pending_dm_peers(resolver, blocks, evt.user_id, &mut recipients).await?;
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

fn dm_cleared_envelope(evt: &DmClearedDto) -> (Vec<Uuid>, serde_json::Value) {
    let allowed = [evt.sender_id, evt.recipient_id];
    let recipients = unique(evt.recipients.iter().copied().filter(|r| allowed.contains(r)));
    let data = json!({
        "actorId": evt.sender_id,
        "participants": [evt.sender_id, evt.recipient_id],
        "scope": evt.scope,
        "clearedAt": evt.cleared_at,
    });
    (recipients, data)
}

fn unique<I: IntoIterator<Item = Uuid>>(iter: I) -> Vec<Uuid> {
    let mut seen = HashSet::new();
    iter.into_iter().filter(|u| seen.insert(*u)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_for_is_stripped_from_the_reply() {
        let hider = Uuid::new_v4();
        let reply = json!({ "id": "r", "content": "secret", "hiddenFor": [hider] });
        let (visible, hidden_for) = split_hidden_reply(Some(&reply));
        assert_eq!(hidden_for, vec![hider]);
        assert!(visible.unwrap().get("hiddenFor").is_none());
    }

    #[test]
    fn a_reply_without_hidden_for_is_visible_to_everyone() {
        let reply = json!({ "id": "r", "content": "hi" });
        let (visible, hidden_for) = split_hidden_reply(Some(&reply));
        assert!(hidden_for.is_empty());
        assert_eq!(visible.unwrap()["content"], "hi");
        assert_eq!(split_hidden_reply(None), (None, Vec::new()));
    }

    #[test]
    fn a_cleared_conversation_reaches_only_its_participants() {
        let (actor, peer, stranger) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let evt = DmClearedDto {
            sender_id: actor,
            recipient_id: peer,
            scope: "everyone".to_string(),
            cleared_at: chrono::Utc::now(),
            recipients: vec![actor, peer, actor, stranger],
        };
        let (recipients, data) = dm_cleared_envelope(&evt);
        assert_eq!(recipients, vec![actor, peer]);
        assert_eq!(data["participants"], json!([actor, peer]));
        assert_eq!(data["scope"], "everyone");
        assert_eq!(data["actorId"], json!(actor));
    }

    #[test]
    fn the_masked_reply_keeps_only_the_id() {
        let masked = deleted_reply(&json!({ "id": "r", "content": "secret", "authorId": "a" }));
        assert_eq!(masked, json!({ "id": "r", "content": "", "hasAttachments": false, "deleted": true }));
    }

    mod pairs {
        use std::sync::Mutex;

        use async_trait::async_trait;
        use chrono::{DateTime, Utc};

        use super::*;
        use crate::ports::blocks::{BlockSets, RelationDirectory, RelationLookupError};

        #[derive(Default)]
        struct Directory {
            removed: Mutex<Vec<(Uuid, Uuid)>>,
        }

        #[async_trait]
        impl RecipientDirectory for Directory {
            async fn community_members(&self, _: Uuid) -> Result<Vec<Uuid>, RepositoryError> { Ok(vec![]) }
            async fn user_name(&self, _: Uuid) -> Result<Option<String>, RepositoryError> { Ok(None) }
            async fn user_name_versioned(&self, _: Uuid) -> Result<Option<(String, DateTime<Utc>)>, RepositoryError> { Ok(None) }
            async fn rename_recipients(&self, _: Uuid) -> Result<Vec<Uuid>, RepositoryError> { Ok(vec![]) }
            async fn profile_recipients(&self, _: Uuid) -> Result<Vec<Uuid>, RepositoryError> { Ok(vec![]) }
            async fn interested_in(&self, _: Uuid) -> Result<Vec<Uuid>, RepositoryError> { Ok(vec![]) }
            async fn pending_dm_peers(&self, _: Uuid) -> Result<Vec<Uuid>, RepositoryError> { Ok(vec![]) }
        }

        #[async_trait]
        impl RealtimeProjection for Directory {
            async fn upsert_user_name(&self, _: Uuid, _: &str, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
            async fn add_friend_edge(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
            async fn add_pending_friend_edge(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
            async fn remove_friend_edge(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
            async fn apply_block(&self, _: Uuid, _: Uuid, _: bool, _: DateTime<Utc>) -> Result<bool, RepositoryError> { Ok(true) }
            async fn record_dm_pair(&self, _: Uuid, _: Uuid) -> Result<(), RepositoryError> { Ok(()) }
            async fn remove_dm_pair(&self, a: Uuid, b: Uuid) -> Result<(), RepositoryError> {
                self.removed.lock().unwrap().push((a, b));
                Ok(())
            }
            async fn add_community_member(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
            async fn remove_community_member(&self, _: Uuid, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
            async fn remove_community(&self, _: Uuid, _: DateTime<Utc>) -> Result<(), RepositoryError> { Ok(()) }
            async fn set_appear_offline(&self, _: Uuid, _: bool) -> Result<(), RepositoryError> { Ok(()) }
            async fn appear_offline(&self, _: Uuid) -> Result<bool, RepositoryError> { Ok(false) }
        }

        struct Relations {
            peers: Vec<Uuid>,
        }

        #[async_trait]
        impl RelationDirectory for Relations {
            async fn block_relations(&self, _: Uuid, _: &[Uuid]) -> Result<BlockSets, RelationLookupError> {
                Ok(BlockSets::default())
            }
            async fn dm_peers(&self, _: Uuid, candidates: &[Uuid]) -> Result<Vec<Uuid>, RelationLookupError> {
                Ok(candidates.iter().copied().filter(|c| self.peers.contains(c)).collect())
            }
        }

        #[derive(Default)]
        struct Captured {
            sent: Mutex<Vec<serde_json::Value>>,
            to: Mutex<Vec<Vec<Uuid>>>,
        }

        #[async_trait]
        impl RealtimePublisher for Captured {
            async fn publish_to_users(&self, recipients: Vec<Uuid>, envelope: &Envelope<'_>) -> Result<(), PublishError> {
                self.sent.lock().unwrap().push(serde_json::to_value(envelope).unwrap());
                self.to.lock().unwrap().push(recipients);
                Ok(())
            }
        }

        #[tokio::test]
        async fn sends_a_skip_to_channel_members_who_get_no_listen_receipt() {
            let publisher = std::sync::Arc::new(Captured::default());
            let dispatch = DispatchEvent::new(
                std::sync::Arc::new(Directory::default()),
                publisher.clone(),
                std::sync::Arc::new(Relations { peers: vec![] }),
            );
            let (listener, author, other) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
            let payload = serde_json::to_vec(&json!({
                "message_id": Uuid::new_v4(),
                "listener_id": listener,
                "listened_at": Utc::now(),
                "recipients": [author, listener],
                "skipped_ids": [other],
            }))
            .unwrap();
            dispatch
                .execute_sequenced(
                    "rift.events.message.p1.voice.listened",
                    &payload,
                    Some(EventSequence { partition: "messaging:c".to_string(), seq: 5, members: vec![] }),
                )
                .await
                .unwrap();
            let sent = publisher.sent.lock().unwrap();
            let to = publisher.to.lock().unwrap();
            assert_eq!(sent[0]["type"], json!("voice.listened"));
            assert_eq!(sent[1]["type"], json!("sequence.skip"));
            assert_eq!(sent[1]["seq"], json!(5));
            assert_eq!(to[1], vec![other]);
        }

        #[tokio::test]
        async fn sends_a_skip_to_the_rest_of_the_channel_when_a_message_is_hidden() {
            let publisher = std::sync::Arc::new(Captured::default());
            let dispatch = DispatchEvent::new(
                std::sync::Arc::new(Directory::default()),
                publisher.clone(),
                std::sync::Arc::new(Relations { peers: vec![] }),
            );
            let (hider, other) = (Uuid::new_v4(), Uuid::new_v4());
            let payload = serde_json::to_vec(&json!({
                "user_id": hider,
                "message_id": Uuid::new_v4(),
                "hidden_at": Utc::now(),
                "skipped_ids": [other, hider],
            }))
            .unwrap();
            dispatch
                .execute_sequenced(
                    "rift.events.message.p3.message.hidden",
                    &payload,
                    Some(EventSequence { partition: "messaging:c".to_string(), seq: 7, members: vec![] }),
                )
                .await
                .unwrap();
            let sent = publisher.sent.lock().unwrap();
            let to = publisher.to.lock().unwrap();
            assert_eq!(sent[0]["type"], json!("message.hidden"));
            assert_eq!(sent[0]["seq"], json!(7));
            assert_eq!(to[0], vec![hider]);
            assert_eq!(sent[1]["type"], json!("sequence.skip"));
            assert_eq!(sent[1]["seq"], json!(7));
            assert_eq!(to[1], vec![other]);
        }

        #[tokio::test]
        async fn sends_a_skip_to_channel_members_who_hid_the_edited_message() {
            let publisher = std::sync::Arc::new(Captured::default());
            let dispatch = DispatchEvent::new(
                std::sync::Arc::new(Directory::default()),
                publisher.clone(),
                std::sync::Arc::new(Relations { peers: vec![] }),
            );
            let (reader, hider) = (Uuid::new_v4(), Uuid::new_v4());
            let payload = serde_json::to_vec(&json!({
                "message_id": Uuid::new_v4(),
                "channel_id": Uuid::new_v4(),
                "new_content": "edited",
                "occurred_at": Utc::now(),
                "recipient_ids": [reader],
                "skipped_ids": [hider],
            }))
            .unwrap();
            dispatch
                .execute_sequenced(
                    "rift.events.message.edited",
                    &payload,
                    Some(EventSequence { partition: "messaging:c".to_string(), seq: 4, members: vec![] }),
                )
                .await
                .unwrap();
            let sent = publisher.sent.lock().unwrap();
            let to = publisher.to.lock().unwrap();
            assert_eq!(sent[0]["type"], json!("message.edited"));
            assert_eq!(to[0], vec![reader]);
            assert_eq!(sent[1]["type"], json!("sequence.skip"));
            assert_eq!(sent[1]["seq"], json!(4));
            assert_eq!(to[1], vec![hider]);
        }

        #[tokio::test]
        async fn tells_members_who_got_nothing_to_skip_the_sequence() {
            let publisher = std::sync::Arc::new(Captured::default());
            let dispatch = DispatchEvent::new(
                std::sync::Arc::new(Directory::default()),
                publisher.clone(),
                std::sync::Arc::new(Relations { peers: vec![] }),
            );
            let (user, peer) = (Uuid::new_v4(), Uuid::new_v4());
            let payload = serde_json::to_vec(&json!({
                "user_id": user,
                "peer_id": peer,
                "hidden": true,
                "occurred_at": Utc::now(),
            }))
            .unwrap();
            dispatch
                .execute_sequenced(
                    "rift.events.user.dm_conversation.hidden_changed",
                    &payload,
                    Some(EventSequence { partition: "messaging:p".to_string(), seq: 9, members: vec![user, peer] }),
                )
                .await
                .unwrap();
            let sent = publisher.sent.lock().unwrap();
            let to = publisher.to.lock().unwrap();
            assert_eq!(sent.len(), 2);
            assert_eq!(sent[0]["type"], json!("dm_conversation.hidden_changed"));
            assert_eq!(to[0], vec![user]);
            assert_eq!(sent[1]["type"], json!("sequence.skip"));
            assert_eq!(sent[1]["seq"], json!(9));
            assert_eq!(to[1], vec![peer]);
        }

        #[tokio::test]
        async fn stamps_every_envelope_with_the_event_sequence() {
            let publisher = std::sync::Arc::new(Captured::default());
            let dispatch = DispatchEvent::new(
                std::sync::Arc::new(Directory::default()),
                publisher.clone(),
                std::sync::Arc::new(Relations { peers: vec![] }),
            );
            let partition = "messaging:p".to_string();
            let payload = serde_json::to_vec(&json!({
                "user_id": Uuid::new_v4(),
                "peer_id": Uuid::new_v4(),
                "hidden": true,
                "occurred_at": Utc::now(),
            }))
            .unwrap();
            dispatch
                .execute_sequenced(
                    "rift.events.user.dm_conversation.hidden_changed",
                    &payload,
                    Some(EventSequence { partition: partition.clone(), seq: 5, members: vec![] }),
                )
                .await
                .unwrap();
            let sent = publisher.sent.lock().unwrap();
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0]["seq"], json!(5));
            assert_eq!(sent[0]["partition"], json!(partition));
        }

        #[tokio::test]
        async fn forgets_a_pair_left_without_history() {
            let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
            let directory = Directory::default();
            forget_pair_without_history(&directory, &Relations { peers: vec![] }, a, b).await.unwrap();
            assert_eq!(*directory.removed.lock().unwrap(), vec![(a, b)]);
        }

        #[tokio::test]
        async fn keeps_a_pair_that_still_has_a_relation() {
            let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
            let directory = Directory::default();
            forget_pair_without_history(&directory, &Relations { peers: vec![b] }, a, b).await.unwrap();
            assert!(directory.removed.lock().unwrap().is_empty());
        }
    }
}
