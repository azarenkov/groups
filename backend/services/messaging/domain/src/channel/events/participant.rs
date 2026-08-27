use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{channel::id::ChannelId, server::id::ServerId};
use domain::{event::EventId, user::id::UserId};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaveReason {
    ClientInitiated,
    Kicked,
    DuplicateIdentity,
    ServerShutdown,
    RoomClosed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSource {
    Camera,
    Screen,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionAbortReason {
    SignalClose,
    MediaFailure,
    ConnectionTimeout,
    StateMismatch,
    JoinFailure,
    Unknown,
    Other,
}


#[derive(Debug)]
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
            id: EventId::generate(),
            channel_id,
            server_id,
            user_id,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct ParticipantLeft {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub reason: LeaveReason,
    pub occurred_at: DateTime<Utc>,
}

impl ParticipantLeft {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        reason: LeaveReason,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            channel_id,
            server_id,
            user_id,
            reason,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
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
            id: EventId::generate(),
            channel_id,
            server_id,
            user_id,
            muted_by,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct ParticipantUnmuted {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub unmuted_by: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl ParticipantUnmuted {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        unmuted_by: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            channel_id,
            server_id,
            user_id,
            unmuted_by,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct ParticipantMediaChanged {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub source: MediaSource,
    pub enabled: bool,
    pub occurred_at: DateTime<Utc>,
}

impl ParticipantMediaChanged {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        source: MediaSource,
        enabled: bool,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            channel_id,
            server_id,
            user_id,
            source,
            enabled,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct TextChannelJoined {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub joined_at: DateTime<Utc>,
    pub recipient_ids: Vec<UserId>,
}

impl TextChannelJoined {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            channel_id,
            server_id,
            user_id,
            joined_at: now,
            recipient_ids: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ParticipantConnectionAborted {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub user_id: UserId,
    pub reason: ConnectionAbortReason,
    pub occurred_at: DateTime<Utc>,
}

impl ParticipantConnectionAborted {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        user_id: UserId,
        reason: ConnectionAbortReason,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            channel_id,
            server_id,
            user_id,
            reason,
            occurred_at: now,
        }
    }
}
