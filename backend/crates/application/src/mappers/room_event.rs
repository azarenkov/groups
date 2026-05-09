use chrono::{DateTime, Utc};

use domain::{
    room::{
        events::{
            RoomEvent,
            participant_events::{
                ParticipantConnectionAbortedEvent, ParticipantJoinedEvent, ParticipantLeftEvent,
            },
            room_events::{RoomFinishedEvent, RoomStartedEvent},
            track_events::{TrackPublishedEvent, TrackUnpublishedEvent},
        },
        id::{InvalidRoomId, RoomId},
    },
    server::id::{InvalidServerId, ServerId},
    shared::value_objects::track_type::{InvalidTrackType, TrackType},
    user::{
        id::{InvalidUserId, UserId},
        name::UserName,
    },
};

use crate::dto::requests::room::events::{
    RoomEventDto, participant_events::ParticipantEventRequest, room_events::RoomEventRequest,
    track_events::TrackEventRequest,
};

#[derive(Debug, thiserror::Error)]
pub enum EventMapperError {
    #[error("Invalid room ID: {0}")]
    InvalidRoomId(#[from] InvalidRoomId),
    #[error("Invalid user ID: {0}")]
    InvalidUserId(#[from] InvalidUserId),
    #[error("Invalid user ID: {0}")]
    InvalidServerId(#[from] InvalidServerId),
    #[error("Invalid track type: {0}")]
    InvalidTrackType(#[from] InvalidTrackType),
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}

pub struct EventMapper;

impl EventMapper {
    pub fn extract_room_id(dto: &RoomEventDto) -> Result<RoomId, EventMapperError> {
        let room_id_str = match dto {
            RoomEventDto::RoomStarted(r) | RoomEventDto::RoomFinished(r) => &r.room_id,
            RoomEventDto::ParticipantJoined(p)
            | RoomEventDto::ParticipantLeft(p)
            | RoomEventDto::ParticipantConnectionAborted(p) => &p.room_id,
            RoomEventDto::TrackPublished(t) | RoomEventDto::TrackUnpublished(t) => &t.room_id,
        };

        Ok(RoomId::from_string(room_id_str)?)
    }

    fn timestamp_to_datetime(timestamp: String) -> Result<DateTime<Utc>, EventMapperError> {
        let timestamp_num = timestamp
            .parse::<i64>()
            .map_err(|_| EventMapperError::InvalidTimestamp)?;
        DateTime::from_timestamp(timestamp_num, 0).ok_or(EventMapperError::InvalidTimestamp)
    }

    fn map_room_started(request: RoomEventRequest) -> Result<RoomStartedEvent, EventMapperError> {
        let created_at = Self::timestamp_to_datetime(request.created_at)?;
        let server_id = request
            .server_id
            .as_ref()
            .map(|s| ServerId::from_string(s))
            .transpose()?;

        Ok(RoomStartedEvent::new(
            request.id,
            RoomId::from_string(&request.room_id)?,
            server_id,
            created_at,
        ))
    }

    fn map_room_finished(request: RoomEventRequest) -> Result<RoomFinishedEvent, EventMapperError> {
        let created_at = Self::timestamp_to_datetime(request.created_at)?;
        let room_id = RoomId::from_string(&request.room_id)?;
        let server_id = request
            .server_id
            .as_ref()
            .map(|s| ServerId::from_string(s))
            .transpose()?;

        Ok(RoomFinishedEvent::new(
            request.id, room_id, server_id, created_at,
        ))
    }

    fn map_participant_joined(
        request: ParticipantEventRequest,
    ) -> Result<ParticipantJoinedEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let user_name = request.user_name.map(UserName::new);
        let room_id = RoomId::from_string(&request.room_id)?;
        let server_id = request
            .server_id
            .as_ref()
            .map(|s| ServerId::from_string(s))
            .transpose()?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;
        let joined_at = Self::timestamp_to_datetime(request.joined_at)?;

        Ok(ParticipantJoinedEvent::new(
            request.id, user_id, user_name, room_id, server_id, joined_at, created_at,
        ))
    }

    fn map_participant_left(
        request: ParticipantEventRequest,
    ) -> Result<ParticipantLeftEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let user_name = request.user_name.map(UserName::new);
        let room_id = RoomId::from_string(&request.room_id)?;
        let server_id = request
            .server_id
            .as_ref()
            .map(|s| ServerId::from_string(s))
            .transpose()?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;
        let joined_at = Self::timestamp_to_datetime(request.joined_at)?;

        Ok(ParticipantLeftEvent::new(
            request.id, user_id, user_name, room_id, server_id, joined_at, created_at,
        ))
    }

    fn map_participant_connection_aborted(
        request: ParticipantEventRequest,
    ) -> Result<ParticipantConnectionAbortedEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let user_name = request.user_name.map(UserName::new);
        let room_id = RoomId::from_string(&request.room_id)?;
        let server_id = request
            .server_id
            .as_ref()
            .map(|s| ServerId::from_string(s))
            .transpose()?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;
        let joined_at = Self::timestamp_to_datetime(request.joined_at)?;

        Ok(ParticipantConnectionAbortedEvent::new(
            request.id, user_id, user_name, room_id, server_id, joined_at, created_at,
        ))
    }

    fn map_track_published(
        request: TrackEventRequest,
    ) -> Result<TrackPublishedEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let user_name = request.user_name.map(UserName::new);
        let room_id = RoomId::from_string(&request.room_id)?;
        let server_id = request
            .server_id
            .as_ref()
            .map(|s| ServerId::from_string(s))
            .transpose()?;
        let track_type = request
            .r#type
            .as_ref()
            .map(|t| TrackType::from_string(t))
            .transpose()?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;

        Ok(TrackPublishedEvent::new(
            request.id, user_id, user_name, room_id, server_id, track_type, created_at,
        ))
    }

    fn map_track_unpublished(
        request: TrackEventRequest,
    ) -> Result<TrackUnpublishedEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let user_name = request.user_name.map(UserName::new);
        let room_id = RoomId::from_string(&request.room_id)?;
        let server_id = request
            .server_id
            .as_ref()
            .map(|s| ServerId::from_string(s))
            .transpose()?;
        let track_type = request
            .r#type
            .as_ref()
            .map(|t| TrackType::from_string(t))
            .transpose()?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;

        Ok(TrackUnpublishedEvent::new(
            request.id, user_id, user_name, room_id, server_id, track_type, created_at,
        ))
    }

    pub fn to_domain_event(dto: RoomEventDto) -> Result<RoomEvent, EventMapperError> {
        match dto {
            RoomEventDto::RoomStarted(request) => {
                Ok(RoomEvent::RoomStarted(Self::map_room_started(request)?))
            }
            RoomEventDto::RoomFinished(request) => {
                Ok(RoomEvent::RoomFinished(Self::map_room_finished(request)?))
            }
            RoomEventDto::ParticipantJoined(request) => Ok(RoomEvent::ParticipantJoined(
                Self::map_participant_joined(request)?,
            )),
            RoomEventDto::ParticipantLeft(request) => Ok(RoomEvent::ParticipantLeft(
                Self::map_participant_left(request)?,
            )),
            RoomEventDto::TrackPublished(request) => Ok(RoomEvent::TrackPublished(
                Self::map_track_published(request)?,
            )),
            RoomEventDto::TrackUnpublished(request) => Ok(RoomEvent::TrackUnpublished(
                Self::map_track_unpublished(request)?,
            )),
            RoomEventDto::ParticipantConnectionAborted(request) => {
                Ok(RoomEvent::ParticipantConnectionAborted(
                    Self::map_participant_connection_aborted(request)?,
                ))
            }
        }
    }
}
