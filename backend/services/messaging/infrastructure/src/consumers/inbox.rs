use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use contracts::events::{
    community::{CommunityInvitationIssuedV1, CommunityInvitationResolvedV1, CommunityMemberV1},
    friendship::{FriendRequestAcceptedV1, FriendRequestSentV1},
};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Event, Lifecycle, spawn_consumer, subject_prefix as normalize_prefix};
use messaging::ports::event_consumer::StartPosition;
use messaging::usecases::inbox::apply_trigger::ApplyInboxTrigger;
use messaging_domain::community::{id::CommunityId, invitation::InvitationId};
use messaging_domain::inbox::{InboxRepository, InboxTrigger};

pub struct InboxFeed {
    pub consumer_name: &'static str,
    pub stream: String,
    pub subjects: Vec<String>,
}

impl InboxFeed {
    pub fn users(prefix: &str, stream: String) -> Self {
        Self {
            consumer_name: "messaging-inbox-feed-users",
            stream,
            subjects: vec![format!("{prefix}.user.*.request_sent"), format!("{prefix}.user.*.request_accepted")],
        }
    }

    pub fn communities(prefix: &str, stream: String) -> Self {
        Self {
            consumer_name: "messaging-inbox-feed-communities",
            stream,
            subjects: vec![
                format!("{prefix}.community.*.invitation_issued"),
                format!("{prefix}.community.*.invitation_resolved"),
                format!("{prefix}.community.*.member_left"),
            ],
        }
    }
}

pub fn trigger_for(
    subject: &str,
    payload: &[u8],
) -> Result<Option<(InboxTrigger, DateTime<Utc>)>, serde_json::Error> {
    let occurred = if subject.ends_with(".request_sent") {
        let evt: FriendRequestSentV1 = serde_json::from_slice(payload)?;
        (InboxTrigger::FriendRequestSent { sender_id: UserId::new(evt.sender_id), receiver_id: UserId::new(evt.receiver_id) }, evt.occurred_at)
    } else if subject.ends_with(".request_accepted") {
        let evt: FriendRequestAcceptedV1 = serde_json::from_slice(payload)?;
        (InboxTrigger::FriendRequestAccepted { accepter_id: UserId::new(evt.accepter_id), sender_id: UserId::new(evt.sender_id) }, evt.occurred_at)
    } else if subject.ends_with(".invitation_issued") {
        let evt: CommunityInvitationIssuedV1 = serde_json::from_slice(payload)?;
        (
            InboxTrigger::InvitationIssued {
                invitation_id: InvitationId::new(evt.invitation_id),
                community_id: CommunityId::new(evt.community_id),
                inviter_id: UserId::new(evt.inviter_id),
                invitee_id: UserId::new(evt.invitee_id),
            },
            evt.occurred_at,
        )
    } else if subject.ends_with(".invitation_resolved") {
        let evt: CommunityInvitationResolvedV1 = serde_json::from_slice(payload)?;
        (
            InboxTrigger::InvitationResolved {
                invitation_id: InvitationId::new(evt.invitation_id),
                invitee_id: UserId::new(evt.invitee_id),
            },
            evt.occurred_at,
        )
    } else if subject.ends_with(".member_left") {
        let evt: CommunityMemberV1 = serde_json::from_slice(payload)?;
        (
            InboxTrigger::MemberRemoved {
                community_id: CommunityId::new(evt.community_id),
                user_id: UserId::new(evt.user_id),
                removed_by: evt.removed_by.map(UserId::new),
            },
            evt.occurred_at,
        )
    } else {
        return Ok(None);
    };
    Ok(Some(occurred))
}

fn source_event_id(event_id: Option<&str>, subject: &str, payload: &[u8]) -> String {
    match event_id {
        Some(id) => id.to_string(),
        None => {
            let mut hasher = DefaultHasher::new();
            subject.hash(&mut hasher);
            payload.hash(&mut hasher);
            format!("hash:{:x}", hasher.finish())
        }
    }
}

pub fn spawn<R>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    feed: InboxFeed,
    started_at: DateTime<Utc>,
    usecase: Arc<ApplyInboxTrigger<R>>,
) -> Arc<AtomicBool>
where
    R: InboxRepository + 'static,
{
    let prefix = normalize_prefix(&subject_prefix);
    let subscribed = Arc::new(AtomicBool::new(false));
    let flag = subscribed.clone();
    let spec = ConsumerSpec::new(&prefix, feed.stream, feed.consumer_name, feed.subjects).starting(StartPosition::Since(started_at));
    let lifecycle = Lifecycle::default().on_subscribed(move || flag.store(true, Ordering::Release));
    spawn_consumer(jetstream, spec, lifecycle, move |event: Event| {
        let usecase = usecase.clone();
        async move {
            let (trigger, occurred_at) = match trigger_for(&event.subject, &event.payload) {
                Ok(Some(occurred)) => occurred,
                Ok(None) => return Ok(()),
                Err(e) => {
                    tracing::warn!(error = %e, subject = %event.subject, "Malformed inbox event; dropping");
                    return Ok(());
                }
            };
            let source = source_event_id(event.event_id.as_deref(), &event.subject, &event.payload);
            usecase.execute(&trigger, occurred_at, &source, Utc::now()).await?;
            Ok(())
        }
    });
    subscribed
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn maps_each_subscribed_event_to_its_trigger() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let now = chrono::Utc::now();
        let sent = serde_json::to_vec(&FriendRequestSentV1 { sender_id: a, receiver_id: b, occurred_at: now }).unwrap();
        assert_eq!(
            trigger_for("rift.events.user.p1.request_sent", &sent).unwrap(),
            Some((InboxTrigger::FriendRequestSent { sender_id: UserId::new(a), receiver_id: UserId::new(b) }, now))
        );
        let kicked = serde_json::to_vec(&CommunityMemberV1 { community_id: c, user_id: a, removed_by: Some(b), occurred_at: now }).unwrap();
        assert_eq!(
            trigger_for("rift.events.community.p2.member_left", &kicked).unwrap(),
            Some((InboxTrigger::MemberRemoved { community_id: CommunityId::new(c), user_id: UserId::new(a), removed_by: Some(UserId::new(b)) }, now))
        );
        let legacy_left = br#"{"community_id":"00000000-0000-0000-0000-000000000001","user_id":"00000000-0000-0000-0000-000000000002","occurred_at":"2026-10-14T10:00:00Z"}"#;
        assert!(matches!(
            trigger_for("rift.events.community.p2.member_left", legacy_left).unwrap(),
            Some((InboxTrigger::MemberRemoved { removed_by: None, .. }, _))
        ));
        assert_eq!(trigger_for("rift.events.user.p1.renamed", b"{}").unwrap(), None);
    }

    #[test]
    fn falls_back_to_a_stable_content_key_without_an_event_id() {
        assert_eq!(source_event_id(Some("evt-1"), "s", b"x"), "evt-1");
        assert_eq!(source_event_id(None, "s", b"x"), source_event_id(None, "s", b"x"));
        assert_ne!(source_event_id(None, "s", b"x"), source_event_id(None, "s", b"y"));
    }
}
