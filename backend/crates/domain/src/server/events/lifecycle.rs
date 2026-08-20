use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    server::{description::ServerDescription, id::ServerId, name::ServerName},
    shared::event::event_id::EventId,
    user::id::UserId,
};

#[derive(Debug, Serialize)]
pub struct ServerCreated {
    pub id: EventId,
    pub server_id: ServerId,
    pub owner_id: UserId,
    pub name: ServerName,
    pub description: Option<ServerDescription>,
    pub occurred_at: DateTime<Utc>,
}

impl ServerCreated {
    pub fn record(
        server_id: ServerId,
        owner_id: UserId,
        name: ServerName,
        description: Option<ServerDescription>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            server_id,
            owner_id,
            name,
            description,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerDeleted {
    pub id: EventId,
    pub server_id: ServerId,
    pub owner_id: UserId,
    pub member_ids: Vec<UserId>,
    pub occurred_at: DateTime<Utc>,
}

impl ServerDeleted {
    pub fn record(
        server_id: ServerId,
        owner_id: UserId,
        member_ids: Vec<UserId>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            server_id,
            owner_id,
            member_ids,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerUpdated {
    pub id: EventId,
    pub server_id: ServerId,
    pub name: ServerName,
    pub description: Option<ServerDescription>,
    pub member_ids: Vec<UserId>,
    pub occurred_at: DateTime<Utc>,
}

impl ServerUpdated {
    pub fn record(
        server_id: ServerId,
        name: ServerName,
        description: Option<ServerDescription>,
        member_ids: Vec<UserId>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            server_id,
            name,
            description,
            member_ids,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerMemberJoined {
    pub id: EventId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl ServerMemberJoined {
    pub fn record(server_id: ServerId, user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            server_id,
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ServerMemberLeft {
    pub id: EventId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl ServerMemberLeft {
    pub fn record(server_id: ServerId, user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::generate(),
            server_id,
            user_id,
            occurred_at: now,
        }
    }
}
