use chrono::{DateTime, Utc};

use crate::{server::events::lifecycle::ServerCreated, server::events::lifecycle::ServerDeleted, server::events::lifecycle::ServerMemberJoined, server::events::lifecycle::ServerMemberLeft, server::events::lifecycle::ServerRolesChanged, server::events::lifecycle::ServerUpdated, server::id::ServerId};

pub mod lifecycle;

#[derive(Debug)]
pub enum ServerEvent {
    Created(ServerCreated),
    Updated(ServerUpdated),
    Deleted(ServerDeleted),
    MemberJoined(ServerMemberJoined),
    MemberLeft(ServerMemberLeft),
    RolesChanged(ServerRolesChanged),
}

impl ServerEvent {
    pub fn server_id(&self) -> &ServerId {
        match self {
            Self::Created(e) => &e.server_id,
            Self::Updated(e) => &e.server_id,
            Self::Deleted(e) => &e.server_id,
            Self::MemberJoined(e) => &e.server_id,
            Self::MemberLeft(e) => &e.server_id,
            Self::RolesChanged(e) => &e.server_id,
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
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => "created",
            Self::Updated(_) => "updated",
            Self::Deleted(_) => "deleted",
            Self::MemberJoined(_) => "member_joined",
            Self::MemberLeft(_) => "member_left",
            Self::RolesChanged(_) => "server.roles.changed",
        }
    }
}
