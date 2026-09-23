use chrono::{DateTime, Utc};

use crate::{community::events::lifecycle::CommunityCreated, community::events::lifecycle::CommunityDeleted, community::events::lifecycle::CommunityMemberJoined, community::events::lifecycle::CommunityMemberLeft, community::events::lifecycle::CommunityOwnerChanged, community::events::lifecycle::CommunityRolesChanged, community::events::lifecycle::CommunityUpdated, community::id::CommunityId};

pub mod lifecycle;

#[derive(Debug)]
pub enum CommunityEvent {
    Created(CommunityCreated),
    Updated(CommunityUpdated),
    Deleted(CommunityDeleted),
    MemberJoined(CommunityMemberJoined),
    MemberLeft(CommunityMemberLeft),
    RolesChanged(CommunityRolesChanged),
    OwnerChanged(CommunityOwnerChanged),
}

impl CommunityEvent {
    pub fn community_id(&self) -> &CommunityId {
        match self {
            Self::Created(e) => &e.community_id,
            Self::Updated(e) => &e.community_id,
            Self::Deleted(e) => &e.community_id,
            Self::MemberJoined(e) => &e.community_id,
            Self::MemberLeft(e) => &e.community_id,
            Self::RolesChanged(e) => &e.community_id,
            Self::OwnerChanged(e) => &e.community_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Created(e) => e.occurred_at,
            Self::Updated(e) => e.occurred_at,
            Self::Deleted(e) => e.occurred_at,
            Self::MemberJoined(e) => e.occurred_at,
            Self::MemberLeft(e) => e.occurred_at,
            Self::RolesChanged(e) => e.occurred_at,
            Self::OwnerChanged(e) => e.occurred_at,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => "created",
            Self::Updated(_) => "updated",
            Self::Deleted(_) => "deleted",
            Self::MemberJoined(_) => "member_joined",
            Self::MemberLeft(_) => "member_left",
            Self::RolesChanged(_) => "community.roles.changed",
            Self::OwnerChanged(_) => "owner_changed",
        }
    }
}
