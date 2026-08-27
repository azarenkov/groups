use chrono::{DateTime, Utc};

use crate::{channel::id::ChannelId, channel::kind::ChannelKind, channel::name::ChannelName, channel::visibility::ChannelVisibility, server::id::ServerId};
use domain::{event::EventId, user::id::UserId};

#[derive(Debug)]
pub struct ChannelCreated {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub owner_id: UserId,
    pub name: ChannelName,
    pub kind: ChannelKind,
    pub visibility: ChannelVisibility,
    pub occurred_at: DateTime<Utc>,
    pub recipient_ids: Vec<UserId>,
}

impl ChannelCreated {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        owner_id: UserId,
        name: ChannelName,
        kind: ChannelKind,
        visibility: ChannelVisibility,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            channel_id,
            server_id,
            owner_id,
            name,
            kind,
            visibility,
            occurred_at: now,
            recipient_ids: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ChannelRenamed {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub old_name: ChannelName,
    pub new_name: ChannelName,
    pub occurred_at: DateTime<Utc>,
}

impl ChannelRenamed {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        old_name: ChannelName,
        new_name: ChannelName,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            channel_id,
            server_id,
            old_name,
            new_name,
            occurred_at: now,
        }
    }
}

#[derive(Debug)]
pub struct ChannelDeleted {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub deleted_by: UserId,
    pub occurred_at: DateTime<Utc>,
    pub recipient_ids: Vec<UserId>,
}

impl ChannelDeleted {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        deleted_by: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::generate(),
            channel_id,
            server_id,
            deleted_by,
            occurred_at: now,
            recipient_ids: Vec::new(),
        }
    }
}
