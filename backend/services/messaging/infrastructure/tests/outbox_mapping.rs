use chrono::Utc;
use contracts::events::{
    channel::ChannelCreatedV1, dm_message::DmMessageSentV1, friendship::FriendRequestSentV1,
    message::ChannelMessageSentV1, community::CommunityRolesChangedV1,
};
use domain::{attachment::AttachmentSnapshot, user::id::UserId};
use messaging_domain::{channel::events::ChannelEvent, channel::events::lifecycle::ChannelCreated, channel::id::ChannelId, channel::kind::ChannelKind, channel::name::ChannelName, channel::visibility::ChannelVisibility, dm_message::DmMessage, events::MessagingEvent, friendship::pair::FriendshipPair, message::Message, message::content::MessageContent, community::events::CommunityEvent, community::events::lifecycle::CommunityRolesChanged, community::id::CommunityId};
use messaging_infrastructure::outbox::mapping::to_record;
use ports::outbox::record::AggregateType;

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> T {
    serde_json::from_value(value).expect("payload must match contract")
}

#[test]
fn message_sent_carries_recipients_attachments_and_mentions() {
    let channel_id = ChannelId::generate();
    let author = UserId::generate();
    let recipient = UserId::generate();
    let mention = UserId::generate();
    let mut message = Message::create(
        channel_id,
        author,
        MessageContent::new("hello".to_string()).unwrap(),
        Utc::now(),
    );
    let snapshot = AttachmentSnapshot {
        id: uuid::Uuid::new_v4(),
        file_name: "a.png".to_string(),
        content_type: "image/png".to_string(),
        size_bytes: 10,
    };
    let events: Vec<MessagingEvent> = message
        .pull_events()
        .into_iter()
        .map(|e| {
            MessagingEvent::from(
                e.with_recipients(&[recipient])
                    .with_mentions(&[mention])
                    .with_attachments(std::slice::from_ref(&snapshot)),
            )
        })
        .collect();
    let record = to_record(&events[0], Some("req-1")).unwrap();
    assert_eq!(record.event_type, "sent");
    assert_eq!(record.aggregate_type, AggregateType::Message);
    assert_eq!(record.parent_id, Some(channel_id.uuid()));
    assert_eq!(record.request_id.as_deref(), Some("req-1"));
    let evt: ChannelMessageSentV1 = decode(record.payload);
    assert_eq!(evt.recipient_ids, vec![recipient.uuid()]);
    assert_eq!(evt.mentions, Some(vec![mention.uuid()]));
    assert_eq!(evt.attachments.unwrap()[0]["id"], serde_json::json!(snapshot.id));
    assert_eq!(evt.content, serde_json::json!("hello"));
}

#[test]
fn dm_call_and_invite_facts_are_part_of_the_event() {
    let sender = UserId::generate();
    let recipient = UserId::generate();
    let mut call = DmMessage::create_call(
        sender,
        recipient,
        uuid::Uuid::new_v4(),
        "missed".to_string(),
        Some(42),
        MessageContent::new("call".to_string()).unwrap(),
        Utc::now(),
    );
    let record = to_record(&MessagingEvent::from(call.pull_events().remove(0)), None).unwrap();
    let evt: DmMessageSentV1 = decode(record.payload);
    assert_eq!(evt.call_status.as_deref(), Some("missed"));
    assert_eq!(evt.call_duration_seconds, Some(42));

    let community_id = CommunityId::generate();
    let mut invite = DmMessage::create_community_invite(
        sender,
        recipient,
        community_id,
        MessageContent::new("Rift HQ".to_string()).unwrap(),
        Utc::now(),
    );
    let record = to_record(&MessagingEvent::from(invite.pull_events().remove(0)), None).unwrap();
    assert_eq!(record.aggregate_type, AggregateType::DmMessage);
    let evt: DmMessageSentV1 = decode(record.payload);
    assert_eq!(evt.invite_community_id, Some(community_id.uuid()));
    assert_eq!(evt.invite_community_name.as_deref(), Some("Rift HQ"));
}

#[test]
fn channel_created_maps_recipients_and_parent_community() {
    let channel_id = ChannelId::generate();
    let community_id = CommunityId::generate();
    let owner = UserId::generate();
    let member = UserId::generate();
    let event = ChannelEvent::Created(ChannelCreated::record(
        channel_id,
        community_id,
        owner,
        ChannelName::new("general".to_string()),
        ChannelKind::Text,
        ChannelVisibility::Public,
        Utc::now(),
    ))
    .with_recipients(&[member]);
    let record = to_record(&MessagingEvent::from(event), None).unwrap();
    assert_eq!(record.event_type, "created");
    assert_eq!(record.aggregate_type, AggregateType::Channel);
    assert_eq!(record.parent_type, Some(AggregateType::Community));
    let evt: ChannelCreatedV1 = decode(record.payload);
    assert_eq!(evt.recipient_ids, vec![member.uuid()]);
    assert_eq!(evt.name, serde_json::json!("general"));
}

#[test]
fn roles_changed_keeps_its_subject_and_recipients() {
    let community_id = CommunityId::generate();
    let member = UserId::generate();
    let event = CommunityEvent::RolesChanged(CommunityRolesChanged::record(community_id, vec![member], Utc::now()));
    let record = to_record(&MessagingEvent::from(event), None).unwrap();
    assert_eq!(record.event_type, "community.roles.changed");
    let evt: CommunityRolesChangedV1 = decode(record.payload);
    assert_eq!(evt.recipients, vec![member.uuid()]);
}

#[test]
fn friend_request_maps_both_parties() {
    let a = UserId::generate();
    let b = UserId::generate();
    let mut pair = FriendshipPair::request(a, b, Utc::now()).unwrap();
    let record = to_record(&MessagingEvent::from(pair.pull_events().remove(0)), None).unwrap();
    assert_eq!(record.event_type, "request_sent");
    assert_eq!(record.aggregate_id, a.uuid());
    assert_eq!(record.parent_id, Some(b.uuid()));
    let evt: FriendRequestSentV1 = decode(record.payload);
    assert_eq!((evt.sender_id, evt.receiver_id), (a.uuid(), b.uuid()));
}
