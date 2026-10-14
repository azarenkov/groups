use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::community::{id::CommunityId, invitation::InvitationId};

pub const INBOX_PAGE_DEFAULT: i64 = 30;
pub const INBOX_PAGE_MAX: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InboxRetention {
    pub max_age: Duration,
    pub max_per_user: i64,
}

impl InboxRetention {
    pub const KEPT_UNTIL_RESOLVED: &'static [&'static str] = &[NotificationKind::COMMUNITY_INVITATION];

    pub fn expired_before(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - self.max_age
    }
}

impl Default for InboxRetention {
    fn default() -> Self {
        Self { max_age: Duration::days(30), max_per_user: 200 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InboxTrigger {
    InvitationIssued {
        invitation_id: InvitationId,
        community_id: CommunityId,
        inviter_id: UserId,
        invitee_id: UserId,
    },
    InvitationResolved {
        invitation_id: InvitationId,
        invitee_id: UserId,
    },
    MemberRemoved {
        community_id: CommunityId,
        user_id: UserId,
        removed_by: Option<UserId>,
    },
    FriendRequestSent {
        sender_id: UserId,
        receiver_id: UserId,
    },
    FriendRequestAccepted {
        accepter_id: UserId,
        sender_id: UserId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationKind {
    CommunityInvitation {
        invitation_id: InvitationId,
        community_id: CommunityId,
        inviter_id: UserId,
    },
    RemovedFromCommunity {
        community_id: CommunityId,
        removed_by: UserId,
    },
    FriendRequestReceived {
        from: UserId,
    },
    FriendRequestAccepted {
        by: UserId,
    },
}

impl NotificationKind {
    pub const COMMUNITY_INVITATION: &'static str = "community_invitation";

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CommunityInvitation { .. } => Self::COMMUNITY_INVITATION,
            Self::RemovedFromCommunity { .. } => "removed_from_community",
            Self::FriendRequestReceived { .. } => "friend_request_received",
            Self::FriendRequestAccepted { .. } => "friend_request_accepted",
        }
    }

    pub fn actor_id(&self) -> &UserId {
        match self {
            Self::CommunityInvitation { inviter_id, .. } => inviter_id,
            Self::RemovedFromCommunity { removed_by, .. } => removed_by,
            Self::FriendRequestReceived { from } => from,
            Self::FriendRequestAccepted { by } => by,
        }
    }

    pub fn community_id(&self) -> Option<&CommunityId> {
        match self {
            Self::CommunityInvitation { community_id, .. } | Self::RemovedFromCommunity { community_id, .. } => {
                Some(community_id)
            }
            _ => None,
        }
    }

    pub fn invitation_id(&self) -> Option<&InvitationId> {
        match self {
            Self::CommunityInvitation { invitation_id, .. } => Some(invitation_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub id: Uuid,
    pub recipient_id: UserId,
    pub kind: NotificationKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InboxEffect {
    Deliver(Notification),
    WithdrawInvitation {
        recipient_id: UserId,
        invitation_id: InvitationId,
    },
}

impl InboxTrigger {
    pub fn effect(&self, now: DateTime<Utc>) -> Option<InboxEffect> {
        let deliver = |recipient_id: UserId, kind: NotificationKind| {
            Some(InboxEffect::Deliver(Notification { id: Uuid::new_v4(), recipient_id, kind, created_at: now }))
        };
        match self {
            Self::InvitationIssued { invitation_id, community_id, inviter_id, invitee_id } => deliver(
                *invitee_id,
                NotificationKind::CommunityInvitation {
                    invitation_id: *invitation_id,
                    community_id: *community_id,
                    inviter_id: *inviter_id,
                },
            ),
            Self::InvitationResolved { invitation_id, invitee_id } => Some(InboxEffect::WithdrawInvitation {
                recipient_id: *invitee_id,
                invitation_id: *invitation_id,
            }),
            Self::MemberRemoved { community_id, user_id, removed_by } => match removed_by {
                Some(remover) if remover != user_id => deliver(
                    *user_id,
                    NotificationKind::RemovedFromCommunity { community_id: *community_id, removed_by: *remover },
                ),
                _ => None,
            },
            Self::FriendRequestSent { sender_id, receiver_id } => {
                deliver(*receiver_id, NotificationKind::FriendRequestReceived { from: *sender_id })
            }
            Self::FriendRequestAccepted { accepter_id, sender_id } => {
                deliver(*sender_id, NotificationKind::FriendRequestAccepted { by: *accepter_id })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InboxItemKind {
    CommunityInvitation {
        invitation_id: InvitationId,
        community_id: CommunityId,
        community_name: String,
    },
    RemovedFromCommunity {
        community_id: CommunityId,
        community_name: String,
    },
    FriendRequestReceived,
    FriendRequestAccepted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InboxItem {
    pub id: Uuid,
    pub kind: InboxItemKind,
    pub actor_id: UserId,
    pub actor_name: String,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InboxCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

impl InboxCursor {
    pub fn of(item: &InboxItem) -> Self {
        Self { created_at: item.created_at, id: item.id }
    }

    pub fn encode(&self) -> String {
        format!("{}_{}", self.created_at.timestamp_micros(), self.id)
    }

    pub fn decode(raw: &str) -> Option<Self> {
        let (micros, id) = raw.split_once('_')?;
        Some(Self {
            created_at: DateTime::from_timestamp_micros(micros.parse().ok()?)?,
            id: Uuid::parse_str(id).ok()?,
        })
    }
}

pub fn clamp_inbox_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(INBOX_PAGE_DEFAULT).clamp(1, INBOX_PAGE_MAX)
}

#[async_trait]
pub trait InboxRepository: Send + Sync {
    async fn deliver(&self, notification: &Notification, source_event_id: &str) -> Result<bool, RepositoryError>;
    async fn withdraw_invitation(&self, recipient_id: &UserId, invitation_id: &InvitationId) -> Result<bool, RepositoryError>;
    async fn list(&self, user_id: &UserId, before: Option<&InboxCursor>, limit: i64) -> Result<Vec<InboxItem>, RepositoryError>;
    async fn unread_count(&self, user_id: &UserId) -> Result<i64, RepositoryError>;
    async fn mark_read(&self, user_id: &UserId, ids: Option<&[Uuid]>, now: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn expire(&self, before: DateTime<Utc>, kept_kinds: &[&str], limit: i64) -> Result<u64, RepositoryError>;
    async fn trim(&self, keep_per_user: i64, kept_kinds: &[&str], limit: i64) -> Result<u64, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> UserId {
        UserId::new(Uuid::new_v4())
    }

    #[test]
    fn invitations_land_in_the_invitee_inbox_and_resolutions_withdraw_them() {
        let (inviter, invitee) = (user(), user());
        let invitation_id = InvitationId::generate();
        let community_id = CommunityId::new(Uuid::new_v4());
        let issued = InboxTrigger::InvitationIssued { invitation_id, community_id, inviter_id: inviter, invitee_id: invitee };
        match issued.effect(Utc::now()) {
            Some(InboxEffect::Deliver(n)) => {
                assert_eq!(n.recipient_id, invitee);
                assert_eq!(n.kind, NotificationKind::CommunityInvitation { invitation_id, community_id, inviter_id: inviter });
            }
            other => panic!("unexpected {other:?}"),
        }
        let resolved = InboxTrigger::InvitationResolved { invitation_id, invitee_id: invitee };
        assert_eq!(
            resolved.effect(Utc::now()),
            Some(InboxEffect::WithdrawInvitation { recipient_id: invitee, invitation_id })
        );
    }

    #[test]
    fn only_removals_by_someone_else_notify_the_removed_member() {
        let (member, moderator) = (user(), user());
        let community_id = CommunityId::new(Uuid::new_v4());
        let kicked = InboxTrigger::MemberRemoved { community_id, user_id: member, removed_by: Some(moderator) };
        assert!(matches!(kicked.effect(Utc::now()), Some(InboxEffect::Deliver(n)) if n.recipient_id == member));
        let left = InboxTrigger::MemberRemoved { community_id, user_id: member, removed_by: None };
        assert_eq!(left.effect(Utc::now()), None);
        let self_removed = InboxTrigger::MemberRemoved { community_id, user_id: member, removed_by: Some(member) };
        assert_eq!(self_removed.effect(Utc::now()), None);
    }

    #[test]
    fn friend_requests_notify_the_receiver_and_acceptances_notify_the_sender() {
        let (sender, receiver) = (user(), user());
        let sent = InboxTrigger::FriendRequestSent { sender_id: sender, receiver_id: receiver };
        assert!(matches!(
            sent.effect(Utc::now()),
            Some(InboxEffect::Deliver(n)) if n.recipient_id == receiver && n.kind == NotificationKind::FriendRequestReceived { from: sender }
        ));
        let accepted = InboxTrigger::FriendRequestAccepted { accepter_id: receiver, sender_id: sender };
        assert!(matches!(
            accepted.effect(Utc::now()),
            Some(InboxEffect::Deliver(n)) if n.recipient_id == sender && n.kind == NotificationKind::FriendRequestAccepted { by: receiver }
        ));
    }

    #[test]
    fn retention_keeps_a_month_of_notifications_and_pending_invitations() {
        let retention = InboxRetention::default();
        let now = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        assert_eq!(retention.expired_before(now), now - Duration::days(30));
        assert_eq!(retention.max_per_user, 200);
        let invitation = NotificationKind::CommunityInvitation {
            invitation_id: InvitationId::generate(),
            community_id: CommunityId::new(Uuid::new_v4()),
            inviter_id: user(),
        };
        assert!(InboxRetention::KEPT_UNTIL_RESOLVED.contains(&invitation.as_str()));
        assert!(!InboxRetention::KEPT_UNTIL_RESOLVED.contains(&NotificationKind::FriendRequestAccepted { by: user() }.as_str()));
    }

    #[test]
    fn cursors_round_trip_and_reject_garbage() {
        let cursor = InboxCursor { created_at: DateTime::from_timestamp_micros(1_700_000_000_123_456).unwrap(), id: Uuid::new_v4() };
        assert_eq!(InboxCursor::decode(&cursor.encode()), Some(cursor));
        assert_eq!(InboxCursor::decode("nope"), None);
    }
}
