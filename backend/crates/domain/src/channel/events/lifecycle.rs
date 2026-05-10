use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    channel::{channel_type::ChannelType, id::ChannelId, name::ChannelName},
    server::id::ServerId,
    shared::event::event_id::EventId,
    user::id::UserId,
};

#[derive(Debug, Serialize)]
pub struct ChannelCreated {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub owner_id: UserId,
    pub name: ChannelName,
    pub channel_type: ChannelType,
    pub occurred_at: DateTime<Utc>,
}

impl ChannelCreated {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        owner_id: UserId,
        name: ChannelName,
        channel_type: ChannelType,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::new(),
            channel_id,
            server_id,
            owner_id,
            name,
            channel_type,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
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
            id: EventId::new(),
            channel_id,
            server_id,
            old_name,
            new_name,
            occurred_at: now,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChannelDeleted {
    pub id: EventId,
    pub channel_id: ChannelId,
    pub server_id: ServerId,
    pub deleted_by: UserId,
    pub occurred_at: DateTime<Utc>,
}

impl ChannelDeleted {
    pub fn record(
        channel_id: ChannelId,
        server_id: ServerId,
        deleted_by: UserId,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: EventId::new(),
            channel_id,
            server_id,
            deleted_by,
            occurred_at: now,
        }
    }
}
