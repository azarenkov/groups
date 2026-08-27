use chrono::Utc;
use messaging_domain::{channel::events::ChannelEvent, channel::events::lifecycle::ChannelCreated, channel::events::lifecycle::ChannelDeleted, channel::events::lifecycle::ChannelRenamed, channel::events::participant::ConnectionAbortReason, channel::events::participant::LeaveReason, channel::events::participant::MediaSource, channel::events::participant::ParticipantConnectionAborted, channel::events::participant::ParticipantJoined, channel::events::participant::ParticipantLeft, channel::events::participant::ParticipantMediaChanged, channel::events::participant::ParticipantMuted, channel::events::participant::ParticipantUnmuted, channel::events::participant::TextChannelJoined, channel::id::ChannelId, channel::kind::ChannelKind, channel::name::ChannelName, channel::visibility::ChannelVisibility, dm_message::events::DmMessageDomainEvent, dm_message::events::lifecycle::DmMessageDeleted, dm_message::events::lifecycle::DmMessageEdited, dm_message::events::lifecycle::DmMessageSent, dm_message::id::DmMessageId, friendship::events::FriendshipDomainEvent, friendship::events::lifecycle::FriendRequestAccepted, friendship::events::lifecycle::FriendRequestSent, friendship::events::lifecycle::FriendshipRemoved, message::content::MessageContent, message::events::MessageDomainEvent, message::events::lifecycle::MessageDeleted, message::events::lifecycle::MessageEdited, message::events::lifecycle::MessageSent, message::id::MessageId, server::description::ServerDescription, server::events::ServerEvent, server::events::lifecycle::ServerCreated, server::events::lifecycle::ServerDeleted, server::events::lifecycle::ServerMemberJoined, server::events::lifecycle::ServerMemberLeft, server::events::lifecycle::ServerUpdated, server::id::ServerId, server::name::ServerName};
use identity_domain::user::events::{UserEvent, lifecycle::{UserRegistered, UserRenamed}};
use domain::{shared::outbox::to_outbox::ToOutboxRecord, user::email::Email, user::id::UserId, user::name::UserName};
use serde_json::Value;

fn payload<E: ToOutboxRecord>(event: &E) -> Value {
    event.to_outbox_record().expect("outbox record").payload
}

fn decode<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("contract must accept producer payload")
}

#[test]
fn user_events_match_contracts() {
    let user_id = UserId::generate();
    let now = Utc::now();

    let registered = UserEvent::Registered(UserRegistered::record(
        user_id,
        UserName::new("tester".to_string()),
        Email::new("t@example.com".to_string()),
        None,
        None,
        now,
    ));
    let evt: contracts::events::user::UserLifecycleV1 = decode(payload(&registered));
    assert_eq!(evt.user_id, user_id.uuid());
    assert_eq!(evt.name.as_deref(), Some("tester"));

    let named: contracts::events::user::UserNameV1 = decode(payload(&registered));
    assert_eq!(named.name, "tester");

    let renamed = UserEvent::Renamed(UserRenamed::record(
        user_id,
        UserName::new("renamed".to_string()),
        now,
    ));
    let evt: contracts::events::user::UserRenamedV1 = decode(payload(&renamed));
    assert_eq!(evt.name, "renamed");
    assert_eq!(evt.occurred_at, now);
}

#[test]
fn friendship_events_match_contracts() {
    let a = UserId::generate();
    let b = UserId::generate();
    let now = Utc::now();

    let sent = FriendshipDomainEvent::RequestSent(FriendRequestSent::record(a, b, now));
    let evt: contracts::events::friendship::FriendRequestSentV1 = decode(payload(&sent));
    assert_eq!((evt.sender_id, evt.receiver_id), (a.uuid(), b.uuid()));

    let accepted =
        FriendshipDomainEvent::RequestAccepted(FriendRequestAccepted::record(b, a, now));
    let evt: contracts::events::friendship::FriendRequestAcceptedV1 = decode(payload(&accepted));
    assert_eq!((evt.accepter_id, evt.sender_id), (b.uuid(), a.uuid()));

    let removed = FriendshipDomainEvent::Removed(FriendshipRemoved::record(a, b, now));
    let evt: contracts::events::friendship::FriendshipRemovedV1 = decode(payload(&removed));
    assert_eq!((evt.user_id, evt.other_id), (a.uuid(), b.uuid()));
}

#[test]
fn server_events_match_contracts() {
    let server_id = ServerId::generate();
    let owner_id = UserId::generate();
    let member_id = UserId::generate();
    let now = Utc::now();

    let created = ServerEvent::Created(ServerCreated::record(
        server_id,
        owner_id,
        ServerName::new("srv".to_string()),
        Some(ServerDescription::new("desc".to_string())),
        now,
    ));
    let evt: contracts::events::server::ServerCreatedV1 = decode(payload(&created));
    assert_eq!(evt.server_id, server_id.uuid());
    assert_eq!(evt.owner_id, owner_id.uuid());

    let updated = ServerEvent::Updated(ServerUpdated::record(
        server_id,
        ServerName::new("srv2".to_string()),
        None,
        vec![owner_id, member_id],
        now,
    ));
    let evt: contracts::events::server::ServerUpdatedV1 = decode(payload(&updated));
    assert_eq!(evt.member_ids, vec![owner_id.uuid(), member_id.uuid()]);

    let deleted = ServerEvent::Deleted(ServerDeleted::record(
        server_id,
        owner_id,
        vec![member_id],
        now,
    ));
    let evt: contracts::events::server::ServerDeletedV1 = decode(payload(&deleted));
    assert_eq!(evt.member_ids, Some(vec![member_id.uuid()]));

    let joined =
        ServerEvent::MemberJoined(ServerMemberJoined::record(server_id, member_id, now));
    let evt: contracts::events::server::ServerMemberV1 = decode(payload(&joined));
    assert_eq!(evt.user_id, member_id.uuid());

    let left = ServerEvent::MemberLeft(ServerMemberLeft::record(server_id, member_id, now));
    let evt: contracts::events::server::ServerMemberV1 = decode(payload(&left));
    assert_eq!(evt.user_id, member_id.uuid());
}

#[test]
fn channel_events_match_contracts() {
    let channel_id = ChannelId::generate();
    let server_id = ServerId::generate();
    let user_id = UserId::generate();
    let now = Utc::now();

    let created = ChannelEvent::Created(ChannelCreated::record(
        channel_id,
        server_id,
        user_id,
        ChannelName::new("general".to_string()),
        ChannelKind::Text,
        ChannelVisibility::Public,
        now,
    ));
    let evt: contracts::events::channel::ChannelCreatedV1 = decode(payload(&created));
    assert_eq!(evt.channel_id, channel_id.uuid());
    assert!(evt.recipient_ids.is_empty());

    let renamed = ChannelEvent::Renamed(ChannelRenamed::record(
        channel_id,
        server_id,
        ChannelName::new("old".to_string()),
        ChannelName::new("new".to_string()),
        now,
    ));
    let _: contracts::events::channel::ChannelRenamedV1 = decode(payload(&renamed));

    let deleted =
        ChannelEvent::Deleted(ChannelDeleted::record(channel_id, server_id, user_id, now));
    let _: contracts::events::channel::ChannelDeletedV1 = decode(payload(&deleted));

    let joined = ChannelEvent::ParticipantJoined(ParticipantJoined::record(
        channel_id, server_id, user_id, now,
    ));
    let _: contracts::events::channel::ParticipantJoinedV1 = decode(payload(&joined));

    let left = ChannelEvent::ParticipantLeft(ParticipantLeft::record(
        channel_id,
        server_id,
        user_id,
        LeaveReason::ClientInitiated,
        now,
    ));
    let _: contracts::events::channel::ParticipantLeftV1 = decode(payload(&left));

    let muted = ChannelEvent::ParticipantMuted(ParticipantMuted::record(
        channel_id, server_id, user_id, user_id, now,
    ));
    let _: contracts::events::channel::ParticipantMutedV1 = decode(payload(&muted));

    let unmuted = ChannelEvent::ParticipantUnmuted(ParticipantUnmuted::record(
        channel_id, server_id, user_id, user_id, now,
    ));
    let _: contracts::events::channel::ParticipantUnmutedV1 = decode(payload(&unmuted));

    let media = ChannelEvent::ParticipantMediaChanged(ParticipantMediaChanged::record(
        channel_id,
        server_id,
        user_id,
        MediaSource::Screen,
        true,
        now,
    ));
    let evt: contracts::events::channel::ParticipantMediaV1 = decode(payload(&media));
    assert_eq!(evt.source, "screen");
    assert!(evt.enabled);

    let aborted = ChannelEvent::ParticipantConnectionAborted(
        ParticipantConnectionAborted::record(
            channel_id,
            server_id,
            user_id,
            ConnectionAbortReason::SignalClose,
            now,
        ),
    );
    let _: contracts::events::channel::ParticipantConnectionAbortedV1 =
        decode(payload(&aborted));

    let text_joined = ChannelEvent::TextJoined(TextChannelJoined::record(
        channel_id, server_id, user_id, now,
    ));
    let evt: contracts::events::channel::TextChannelJoinedV1 = decode(payload(&text_joined));
    assert_eq!(evt.joined_at, now);
}

#[test]
fn message_events_match_contracts_with_enrichment() {
    let message_id = MessageId::generate();
    let channel_id = ChannelId::generate();
    let author_id = UserId::generate();
    let peer_id = UserId::generate();
    let now = Utc::now();

    let sent = MessageDomainEvent::Sent(MessageSent::record(
        message_id,
        channel_id,
        author_id,
        MessageContent::new("hello".to_string()).unwrap(),
        now,
    ));
    let mut records = vec![sent.to_outbox_record().unwrap()];
    messaging::usecases::channel::recipients::inject_recipient_ids(
        &mut records,
        &[author_id, peer_id],
    );
    messaging::usecases::enrichment::inject_field(
        &mut records,
        "attachments",
        serde_json::json!([]),
    );
    let evt: contracts::events::message::ChannelMessageSentV1 =
        decode(records.remove(0).payload);
    assert_eq!(evt.recipient_ids, vec![author_id.uuid(), peer_id.uuid()]);
    assert_eq!(evt.attachments, Some(serde_json::json!([])));

    let edited = MessageDomainEvent::Edited(MessageEdited::record(
        message_id,
        channel_id,
        MessageContent::new("edited".to_string()).unwrap(),
        now,
    ));
    let _: contracts::events::message::ChannelMessageEditedV1 = decode(payload(&edited));

    let deleted =
        MessageDomainEvent::Deleted(MessageDeleted::record(message_id, channel_id, author_id, now));
    let _: contracts::events::message::ChannelMessageDeletedV1 = decode(payload(&deleted));
}

#[test]
fn dm_events_match_contracts() {
    let message_id = DmMessageId::generate();
    let sender_id = UserId::generate();
    let recipient_id = UserId::generate();
    let now = Utc::now();

    let sent = DmMessageDomainEvent::Sent(DmMessageSent::record(
        message_id,
        sender_id,
        recipient_id,
        MessageContent::new("hi".to_string()).unwrap(),
        now,
    ));
    let evt: contracts::events::dm_message::DmMessageSentV1 = decode(payload(&sent));
    assert_eq!(evt.recipient_id, recipient_id.uuid());
    assert!(evt.attachments.is_none());

    let edited = DmMessageDomainEvent::Edited(DmMessageEdited::record(
        message_id,
        sender_id,
        MessageContent::new("hi2".to_string()).unwrap(),
        now,
    ));
    let mut records = vec![edited.to_outbox_record().unwrap()];
    messaging::usecases::enrichment::inject_field(
        &mut records,
        "recipient_id",
        serde_json::json!(recipient_id.uuid()),
    );
    let evt: contracts::events::dm_message::DmMessageEditedV1 =
        decode(records.remove(0).payload);
    assert_eq!(evt.recipient_id, Some(recipient_id.uuid()));

    let deleted = DmMessageDomainEvent::Deleted(DmMessageDeleted::record(
        message_id,
        sender_id,
        recipient_id,
        now,
    ));
    let _: contracts::events::dm_message::DmMessageDeletedV1 = decode(payload(&deleted));
}

#[test]
fn presence_preference_round_trips() {
    let original = contracts::realtime::PresencePreferenceV1 {
        user_id: uuid::Uuid::new_v4(),
        appear_offline: true,
    };
    let bytes = serde_json::to_vec(&original).unwrap();
    let parsed: contracts::realtime::PresencePreferenceV1 =
        serde_json::from_slice(&bytes).unwrap();
    assert_eq!(parsed.user_id, original.user_id);
    assert!(parsed.appear_offline);
}
