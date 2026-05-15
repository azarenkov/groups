use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    channel::{
        events::{
            lifecycle::{ChannelCreated, ChannelDeleted, ChannelRenamed},
            participant::{
                ParticipantConnectionAborted, ParticipantJoined, ParticipantLeft, ParticipantMuted,
                ParticipantUnmuted,
            },
        },
        id::ChannelId,
    },
    server::id::ServerId,
    shared::outbox::{
        outbox_record::{AggregateType, OutboxRecord},
        to_outbox::{OutboxError, ToOutboxRecord},
    },
};

pub mod lifecycle;
pub mod participant;

#[derive(Debug, Serialize)]
pub enum ChannelEvent {
    Created(ChannelCreated),
    Renamed(ChannelRenamed),
    Deleted(ChannelDeleted),
    ParticipantJoined(ParticipantJoined),
    ParticipantLeft(ParticipantLeft),
    ParticipantMuted(ParticipantMuted),
    ParticipantUnmuted(ParticipantUnmuted),
    ParticipantConnectionAborted(ParticipantConnectionAborted),
}

impl ChannelEvent {
    pub fn channel_id(&self) -> &ChannelId {
        match self {
            Self::Created(e) => &e.channel_id,
            Self::Renamed(e) => &e.channel_id,
            Self::Deleted(e) => &e.channel_id,
            Self::ParticipantJoined(e) => &e.channel_id,
            Self::ParticipantLeft(e) => &e.channel_id,
            Self::ParticipantMuted(e) => &e.channel_id,
            Self::ParticipantUnmuted(e) => &e.channel_id,
            Self::ParticipantConnectionAborted(e) => &e.channel_id,
        }
    }

    pub fn server_id(&self) -> &ServerId {
        match self {
            Self::Created(e) => &e.server_id,
            Self::Renamed(e) => &e.server_id,
            Self::Deleted(e) => &e.server_id,
            Self::ParticipantJoined(e) => &e.server_id,
            Self::ParticipantLeft(e) => &e.server_id,
            Self::ParticipantMuted(e) => &e.server_id,
            Self::ParticipantUnmuted(e) => &e.server_id,
            Self::ParticipantConnectionAborted(e) => &e.server_id,
        }
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        match self {
            Self::Created(e) => e.occurred_at,
            Self::Renamed(e) => e.occurred_at,
            Self::Deleted(e) => e.occurred_at,
            Self::ParticipantJoined(e) => e.occurred_at,
            Self::ParticipantLeft(e) => e.occurred_at,
            Self::ParticipantMuted(e) => e.occurred_at,
            Self::ParticipantUnmuted(e) => e.occurred_at,
            Self::ParticipantConnectionAborted(e) => e.occurred_at,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Created(_) => "created",
            Self::Renamed(_) => "renamed",
            Self::Deleted(_) => "deleted",
            Self::ParticipantJoined(_) => "participant.joined",
            Self::ParticipantLeft(_) => "participant.left",
            Self::ParticipantMuted(_) => "participant.muted",
            Self::ParticipantUnmuted(_) => "participant.unmuted",
            Self::ParticipantConnectionAborted(_) => "participant.connection_aborted",
        }
    }
}

impl ToOutboxRecord for ChannelEvent {
    fn to_outbox_record(&self) -> Result<OutboxRecord, OutboxError> {
        Ok(OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: self.channel_id().uuid(),
            aggregate_type: AggregateType::Channel,
            parent_id: Some(self.server_id().uuid()),
            parent_type: Some(AggregateType::Server),
            event_type: self.event_type().to_string(),
            payload: serde_json::to_value(self)?,
            occurred_at: self.occurred_at(),
            request_id: None,
        })
    }
}
