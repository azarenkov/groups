use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    channel::id::ChannelId, server::id::ServerId, shared::event::event_id::EventId,
    user::id::UserId,
};

#[derive(Debug, Serialize)]
pub struct ParticipantJoined {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl ParticipantJoined {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::new(),
            channel_id,
            server_id,
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ParticipantLeft {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl ParticipantLeft {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::new(),
            channel_id,
            server_id,
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ParticipantMuted {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub muted_by: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl ParticipantMuted {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        muted_by: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::new(),
            channel_id,
            server_id,
            user_id,
            muted_by,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ParticipantConnectionAborted {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl ParticipantConnectionAborted {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::new(),
            channel_id,
            server_id,
            user_id,
            occurred_at: now,
        }
    }
}
