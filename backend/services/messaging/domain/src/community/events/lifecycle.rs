use chrono::{DateTime, Utc};

use crate::{community::description::CommunityDescription, community::id::CommunityId, community::name::CommunityName};
use domain::{event::EventId, user::id::UserId};

#[derive(Debug)]
pub struct CommunityCreated {
    pub id: EventId,
    pub community_id: CommunityId,
    pub owner_id: UserId,
    pub name: CommunityName,
    pub description: Option<CommunityDescription>,
    pub occurred_at: DateTime<Utc>,
}

impl CommunityCreated {
    pub fn record(
        community_id: CommunityId,
        owner_id: UserId,
        name: CommunityName,
        description: Option<CommunityDescription>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            community_id,
            owner_id,
            name,
            description,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct CommunityDeleted {
    pub id: EventId,
    pub community_id: CommunityId,
    pub owner_id: UserId,
    pub member_ids: Vec<UserId>,
    pub occurred_at: DateTime<Utc>,
}

impl CommunityDeleted {
    pub fn record(
        community_id: CommunityId,
        owner_id: UserId,
        member_ids: Vec<UserId>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            community_id,
            owner_id,
            member_ids,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct CommunityUpdated {
    pub id: EventId,
    pub community_id: CommunityId,
    pub name: CommunityName,
    pub description: Option<CommunityDescription>,
    pub member_ids: Vec<UserId>,
    pub occurred_at: DateTime<Utc>,
}

impl CommunityUpdated {
    pub fn record(
        community_id: CommunityId,
        name: CommunityName,
        description: Option<CommunityDescription>,
        member_ids: Vec<UserId>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            community_id,
            name,
            description,
            member_ids,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct CommunityMemberJoined {
    pub id: EventId,
    pub community_id: CommunityId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl CommunityMemberJoined {
    pub fn record(community_id: CommunityId, user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            community_id,
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct CommunityRolesChanged {
    pub id: EventId,
    pub community_id: CommunityId,
    pub recipients: Vec<UserId>,
    pub occurred_at: DateTime<Utc>,
}

impl CommunityRolesChanged {
    pub fn record(community_id: CommunityId, recipients: Vec<UserId>, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            community_id,
            recipients,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct CommunityMemberLeft {
    pub id: EventId,
    pub community_id: CommunityId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl CommunityMemberLeft {
    pub fn record(community_id: CommunityId, user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            community_id,
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct CommunityOwnerChanged {
    pub id: EventId,
    pub community_id: CommunityId,
    pub previous_owner_id: UserId,
    pub owner_id: UserId,
    pub member_ids: Vec<UserId>,
    pub occurred_at: DateTime<Utc>,
}

impl CommunityOwnerChanged {
    pub fn record(
        community_id: CommunityId,
        previous_owner_id: UserId,
        owner_id: UserId,
        member_ids: Vec<UserId>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            community_id,
            previous_owner_id,
            owner_id,
            member_ids,
            occurred_at: now,
        }
    }
}
