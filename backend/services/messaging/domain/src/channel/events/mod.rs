use chrono::{DateTime, Utc};

use crate::{channel::events::lifecycle::ChannelCreated, channel::events::lifecycle::ChannelDeleted, channel::events::lifecycle::ChannelRenamed, channel::events::participant::MediaSource, channel::events::participant::ParticipantConnectionAborted, channel::events::participant::ParticipantJoined, channel::events::participant::ParticipantLeft, channel::events::participant::ParticipantMediaChanged, channel::events::participant::ParticipantDeafened, channel::events::participant::ParticipantMuted, channel::events::participant::ParticipantUndeafened, channel::events::participant::ParticipantUnmuted, channel::events::participant::TextChannelJoined, channel::id::ChannelId, community::id::CommunityId};

pub mod lifecycle;
pub mod participant;

#[derive(Debug)]
pub enum ChannelEvent {
    Created(ChannelCreated),
    Renamed(ChannelRenamed),
    Deleted(ChannelDeleted),
    ParticipantJoined(ParticipantJoined),
    ParticipantLeft(ParticipantLeft),
    ParticipantMuted(ParticipantMuted),
    ParticipantUnmuted(ParticipantUnmuted),
    ParticipantDeafened(ParticipantDeafened),
    ParticipantUndeafened(ParticipantUndeafened),
    ParticipantConnectionAborted(ParticipantConnectionAborted),
    ParticipantMediaChanged(ParticipantMediaChanged),
    TextJoined(TextChannelJoined),
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
            Self::ParticipantDeafened(e) => &e.channel_id,
            Self::ParticipantUndeafened(e) => &e.channel_id,
            Self::ParticipantConnectionAborted(e) => &e.channel_id,
            Self::ParticipantMediaChanged(e) => &e.channel_id,
            Self::TextJoined(e) => &e.channel_id,
        }
    }

    pub fn community_id(&self) -> &CommunityId {
        match self {
            Self::Created(e) => &e.community_id,
            Self::Renamed(e) => &e.community_id,
            Self::Deleted(e) => &e.community_id,
            Self::ParticipantJoined(e) => &e.community_id,
            Self::ParticipantLeft(e) => &e.community_id,
            Self::ParticipantMuted(e) => &e.community_id,
            Self::ParticipantUnmuted(e) => &e.community_id,
            Self::ParticipantDeafened(e) => &e.community_id,
            Self::ParticipantUndeafened(e) => &e.community_id,
            Self::ParticipantConnectionAborted(e) => &e.community_id,
            Self::ParticipantMediaChanged(e) => &e.community_id,
            Self::TextJoined(e) => &e.community_id,
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
            Self::ParticipantDeafened(e) => e.occurred_at,
            Self::ParticipantUndeafened(e) => e.occurred_at,
            Self::ParticipantConnectionAborted(e) => e.occurred_at,
            Self::ParticipantMediaChanged(e) => e.occurred_at,
            Self::TextJoined(e) => e.joined_at,
        }
    }

    pub fn departed_user(&self) -> Option<domain::user::id::UserId> {
        match self {
            Self::ParticipantLeft(e) => Some(e.user_id),
            Self::ParticipantConnectionAborted(e) => Some(e.user_id),
            _ => None,
        }
    }

    pub fn screen_stopped_user(&self) -> Option<domain::user::id::UserId> {
        match self {
            Self::ParticipantMediaChanged(e)
                if e.source == MediaSource::Screen && !e.enabled =>
            {
                Some(e.user_id)
            }
            _ => None,
        }
    }

    pub fn screen_started_user(&self) -> Option<domain::user::id::UserId> {
        match self {
            Self::ParticipantMediaChanged(e)
                if e.source == MediaSource::Screen && e.enabled =>
            {
                Some(e.user_id)
            }
            _ => None,
        }
    }

    pub fn with_recipients(mut self, recipient_ids: &[domain::user::id::UserId]) -> Self {
        match &mut self {
            Self::Created(e) => e.recipient_ids = recipient_ids.to_vec(),
            Self::Deleted(e) => e.recipient_ids = recipient_ids.to_vec(),
            Self::TextJoined(e) => e.recipient_ids = recipient_ids.to_vec(),
            _ => {}
        }
        self
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
            Self::ParticipantDeafened(_) => "participant.deafened",
            Self::ParticipantUndeafened(_) => "participant.undeafened",
            Self::ParticipantConnectionAborted(_) => "participant.connection_aborted",
            Self::ParticipantMediaChanged(e) => match (e.source, e.enabled) {
                (MediaSource::Camera, true) => "participant.camera.on",
                (MediaSource::Camera, false) => "participant.camera.off",
                (MediaSource::Screen, true) => "participant.screen.on",
                (MediaSource::Screen, false) => "participant.screen.off",
            },
            Self::TextJoined(_) => "text.joined",
        }
    }
}
