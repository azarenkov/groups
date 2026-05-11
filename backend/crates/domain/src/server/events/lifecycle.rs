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
            id: EventId::new(),
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
    pub occurred_at: DateTime<Utc>,
}

impl ServerDeleted {
    pub fn record(server_id: ServerId, owner_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id: EventId::new(),
            server_id,
            owner_id,
            occurred_at: now,
        }
    }
}
