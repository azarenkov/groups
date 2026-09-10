use chrono::Utc;

use super::{Channel, ChannelError, UnmuteOutcome, kind::ChannelKind, moderation::ModerationGrant, moderation::Publication, name::ChannelName, session_id::SessionId, visibility::ChannelVisibility};
use crate::{channel::events::ChannelEvent, channel::events::participant::LeaveReason, channel::events::participant::MediaSource, server::id::ServerId};
use domain::user::id::UserId;

fn voice_channel(owner: UserId) -> Channel {
    let mut channel = Channel::create(
        ServerId::generate(),
        owner,
        ChannelName::new("voice".to_string()),
        ChannelKind::Voice,
        ChannelVisibility::Public,
        None,
        Utc::now(),
    )
    .unwrap();
    channel.pull_events();
    channel
}

#[test]
fn default_channel_cannot_be_deleted_but_others_can() {
    let owner = UserId::generate();
    let mut general = Channel::create(
        ServerId::generate(),
        owner,
        ChannelName::new("general".to_string()),
        ChannelKind::Text,
        ChannelVisibility::Public,
        None,
        Utc::now(),
    )
    .unwrap()
    .with_default(true);
    assert!(general.is_default());
    assert!(matches!(
        general.pull_events().as_slice(),
        [ChannelEvent::Created(created)] if created.is_default
    ));
    assert_eq!(
        general.delete(owner, ModerationGrant::Owner, Utc::now()).unwrap_err(),
        ChannelError::DefaultChannel
    );
    let mut other = voice_channel(owner);
    assert!(!other.is_default());
    assert!(other.delete(owner, ModerationGrant::Owner, Utc::now()).is_ok());
}

#[test]
fn creator_has_no_authority_without_a_grant() {
    let creator = UserId::generate();
    let other = UserId::generate();
    let mut channel = voice_channel(creator);
    assert_eq!(
        channel.mute(other, creator, ModerationGrant::SelfService, Utc::now()).unwrap_err(),
        ChannelError::NotAllowed
    );
    assert_eq!(
        channel.delete(creator, ModerationGrant::SelfService, Utc::now()).unwrap_err(),
        ChannelError::NotAllowed
    );
    assert!(channel.delete(creator, ModerationGrant::Moderator, Utc::now()).is_ok());
}

#[test]
fn self_mute_then_moderator_mute_keeps_self_mute_and_restricts_publication() {
    let owner = UserId::generate();
    let user = UserId::generate();
    let moderator = UserId::generate();
    let mut channel = voice_channel(owner);
    channel.mute(user, user, ModerationGrant::SelfService, Utc::now()).unwrap();
    assert_eq!(channel.publication_for(&user), Publication::Full);
    assert_eq!(
        channel.mute(user, user, ModerationGrant::SelfService, Utc::now()).unwrap_err(),
        ChannelError::AlreadyMuted
    );
    channel.mute(user, moderator, ModerationGrant::Moderator, Utc::now()).unwrap();
    assert_eq!(channel.publication_for(&user), Publication::Restricted);
    let record = channel.mute_record(&user).unwrap();
    assert_eq!(record.muted_by, moderator);
    assert!(record.self_muted);
}

#[test]
fn moderator_unmute_downgrades_to_self_mute_when_user_was_self_muted() {
    let owner = UserId::generate();
    let user = UserId::generate();
    let moderator = UserId::generate();
    let mut channel = voice_channel(owner);
    channel.mute(user, user, ModerationGrant::SelfService, Utc::now()).unwrap();
    channel.mute(user, moderator, ModerationGrant::Moderator, Utc::now()).unwrap();
    channel.pull_events();
    let outcome = channel.unmute(user, moderator, ModerationGrant::Moderator, Utc::now()).unwrap();
    assert_eq!(outcome, UnmuteOutcome::DowngradedToSelfMute);
    assert_eq!(channel.publication_for(&user), Publication::Full);
    assert_eq!(channel.mute_record(&user).unwrap().muted_by, user);
    assert!(matches!(channel.pull_events().as_slice(), [ChannelEvent::ParticipantMuted(_)]));
}

#[test]
fn self_unmute_under_moderator_mute_clears_only_the_self_mute_flag() {
    let owner = UserId::generate();
    let user = UserId::generate();
    let moderator = UserId::generate();
    let mut channel = voice_channel(owner);
    channel.mute(user, user, ModerationGrant::SelfService, Utc::now()).unwrap();
    channel.mute(user, moderator, ModerationGrant::Moderator, Utc::now()).unwrap();
    channel.pull_events();
    assert_eq!(
        channel.unmute(user, user, ModerationGrant::SelfService, Utc::now()).unwrap(),
        UnmuteOutcome::SelfMuteCleared
    );
    let record = channel.mute_record(&user).unwrap();
    assert_eq!(record.muted_by, moderator);
    assert!(!record.self_muted);
    assert_eq!(channel.publication_for(&user), Publication::Restricted);
    assert!(channel.pull_events().is_empty());
    assert_eq!(
        channel.unmute(user, user, ModerationGrant::SelfService, Utc::now()).unwrap_err(),
        ChannelError::NotAllowed
    );
    assert_eq!(
        channel.unmute(user, moderator, ModerationGrant::Moderator, Utc::now()).unwrap(),
        UnmuteOutcome::Removed
    );
}

#[test]
fn self_service_only_lifts_own_mute() {
    let owner = UserId::generate();
    let user = UserId::generate();
    let moderator = UserId::generate();
    let mut channel = voice_channel(owner);
    channel.mute(user, moderator, ModerationGrant::Moderator, Utc::now()).unwrap();
    assert_eq!(
        channel.unmute(user, user, ModerationGrant::SelfService, Utc::now()).unwrap_err(),
        ChannelError::NotAllowed
    );
    assert_eq!(
        channel.unmute(user, owner, ModerationGrant::Owner, Utc::now()).unwrap(),
        UnmuteOutcome::Removed
    );
    assert!(channel.mute_record(&user).is_none());
    assert_eq!(
        channel.unmute(user, owner, ModerationGrant::Owner, Utc::now()).unwrap_err(),
        ChannelError::NotMuted
    );
}

#[test]
fn voice_sessions_track_presence_and_media() {
    let owner = UserId::generate();
    let user = UserId::generate();
    let mut channel = voice_channel(owner);
    let s1 = SessionId::new("s1".to_string());
    let s2 = SessionId::new("s2".to_string());
    channel.join(user, s1.clone(), Utc::now()).unwrap();
    channel.join(user, s2.clone(), Utc::now()).unwrap();
    assert!(matches!(channel.pull_events().as_slice(), [ChannelEvent::ParticipantJoined(_)]));
    assert_eq!(channel.pull_voice_changes().len(), 2);

    channel.publish_track(user, s1.clone(), MediaSource::Screen, Utc::now()).unwrap();
    assert!(matches!(channel.pull_events().as_slice(), [ChannelEvent::ParticipantMediaChanged(_)]));
    channel.leave(user, s1, LeaveReason::ClientInitiated, Utc::now()).unwrap();
    let events = channel.pull_events();
    assert!(events.iter().any(|e| matches!(e, ChannelEvent::ParticipantMediaChanged(m) if !m.enabled)));
    assert!(!events.iter().any(|e| matches!(e, ChannelEvent::ParticipantLeft(_))));
    channel.leave(user, s2, LeaveReason::ClientInitiated, Utc::now()).unwrap();
    assert!(matches!(channel.pull_events().as_slice(), [ChannelEvent::ParticipantLeft(_)]));
    assert_eq!(
        channel.publish_track(user, SessionId::new("s3".to_string()), MediaSource::Camera, Utc::now()).unwrap_err(),
        ChannelError::ParticipantNotJoined
    );
}
