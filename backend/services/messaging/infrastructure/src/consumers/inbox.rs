use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use contracts::events::{
    community::{CommunityInvitationIssuedV1, CommunityInvitationResolvedV1, CommunityMemberV1},
    friendship::{FriendRequestAcceptedV1, FriendRequestSentV1},
};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use messaging::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use messaging::usecases::inbox::apply_trigger::ApplyInboxTrigger;
use messaging_domain::community::{id::CommunityId, invitation::InvitationId};
use messaging_domain::inbox::{InboxRepository, InboxTrigger};

const MAX_DELIVER: u32 = 20;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const RESUBSCRIBE_DELAY: Duration = Duration::from_secs(5);

pub struct InboxFeed {
    pub consumer_name: &'static str,
    pub stream: String,
    pub subjects: Vec<String>,
    pub dlq_suffix: &'static str,
}

impl InboxFeed {
    pub fn users(prefix: &str, stream: String) -> Self {
        Self {
            consumer_name: "messaging-inbox-users",
            stream,
            subjects: vec![format!("{prefix}.user.*.request_sent"), format!("{prefix}.user.*.request_accepted")],
            dlq_suffix: "messaging.inbox.users",
        }
    }

    pub fn communities(prefix: &str, stream: String) -> Self {
        Self {
            consumer_name: "messaging-inbox-communities",
            stream,
            subjects: vec![
                format!("{prefix}.community.*.invitation_issued"),
                format!("{prefix}.community.*.invitation_resolved"),
                format!("{prefix}.community.*.member_left"),
            ],
            dlq_suffix: "messaging.inbox.communities",
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

pub fn spawn<R>(jetstream: async_nats::jetstream::Context, subject_prefix: String, feed: InboxFeed, usecase: Arc<ApplyInboxTrigger<R>>)
where
    R: InboxRepository + 'static,
{
    let prefix = subject_prefix.trim_end_matches('.').to_string();
    tokio::spawn(async move {
        let consumer = NatsEventConsumer::new(jetstream, feed.stream.clone());
        let mut subjects = feed.subjects.clone();
        let subject = subjects.remove(0);
        loop {
            let opts = SubscribeOptions {
                consumer_name: feed.consumer_name.to_string(),
                subject: subject.clone(),
                start_position: StartPosition::New,
                max_deliver: Some(MAX_DELIVER),
                dlq_subject: Some(format!("{prefix}.dlq.{}", feed.dlq_suffix)),
                extra_subjects: subjects.clone(),
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(stream = %feed.stream, consumer = feed.consumer_name, "Inbox feed subscribed");
                    while let Some(msg) = messages.next().await {
                        let Ok(m) = msg else {
                            tracing::warn!(consumer = feed.consumer_name, "Inbox feed stream error");
                            break;
                        };
                        let (trigger, occurred_at) = match trigger_for(&m.subject, &m.payload) {
                            Ok(Some(occurred)) => occurred,
                            Ok(None) => {
                                let _ = m.ack().await;
                                continue;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, subject = %m.subject, "Malformed inbox event; dropping");
                                let _ = m.ack().await;
                                continue;
                            }
                        };
                        let source = source_event_id(m.event_id.as_deref(), &m.subject, &m.payload);
                        match usecase.execute(&trigger, occurred_at, &source).await {
                            Ok(()) => {
                                let _ = m.ack().await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, subject = %m.subject, "Inbox delivery failed; scheduling retry");
                                let _ = m.retry_after(RETRY_DELAY).await;
                            }
                        }
                    }
                }
                Err(e) => tracing::warn!(error = %e, consumer = feed.consumer_name, "Inbox feed subscribe failed; retrying"),
            }
            tokio::time::sleep(RESUBSCRIBE_DELAY).await;
        }
    });
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
