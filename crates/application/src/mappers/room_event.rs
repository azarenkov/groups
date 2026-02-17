use chrono::{DateTime, Utc};

use domain::{
    room::{
        Room,
        events::{
            RoomEvent,
            participant_events::{ParticipantJoinedEvent, ParticipantLeftEvent},
            room_events::{RoomFinishedEvent, RoomStartedEvent},
            track_events::{TrackPublishedEvent, TrackUnpublishedEvent},
        },
        value_objects::id::{InvalidRoomId, RoomId},
    },
    shared::value_objects::track_type::{InvalidTrackType, TrackType},
    user::value_objects::id::{InvalidUserId, UserId},
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
            RoomEventDto::ParticipantJoined(p) | RoomEventDto::ParticipantLeft(p) => &p.room_id,
            RoomEventDto::TrackPublished(t) | RoomEventDto::TrackUnpublished(t) => &t.room_id,
        };

        Ok(RoomId::from_string(room_id_str)?)
    }

    fn timestamp_to_datetime(timestamp: u64) -> Result<DateTime<Utc>, EventMapperError> {
        DateTime::from_timestamp(timestamp as i64, 0).ok_or(EventMapperError::InvalidTimestamp)
    }

    fn map_room_started(
        request: RoomEventRequest,
        room: &Room,
    ) -> Result<RoomStartedEvent, EventMapperError> {
        let created_at = Self::timestamp_to_datetime(request.created_at)?;

        Ok(RoomStartedEvent::new(
            request.id,
            *room.id(),
            *room.owner_id(),
            *room.server_id(),
            *room.room_type(),
            created_at,
        ))
    }

    fn map_room_finished(
        request: RoomEventRequest,
        room: &Room,
    ) -> Result<RoomFinishedEvent, EventMapperError> {
        let created_at = Self::timestamp_to_datetime(request.created_at)?;

        Ok(RoomFinishedEvent::new(
            request.id,
            *room.id(),
            *room.owner_id(),
            *room.server_id(),
            *room.room_type(),
            created_at,
        ))
    }

    fn map_participant_joined(
        request: ParticipantEventRequest,
        room: &Room,
    ) -> Result<ParticipantJoinedEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;
        let joined_at = Self::timestamp_to_datetime(request.joined_at)?;

        Ok(ParticipantJoinedEvent::new(
            request.id,
            user_id,
            *room.id(),
            joined_at,
            created_at,
        ))
    }

    fn map_participant_left(
        request: ParticipantEventRequest,
        room: &Room,
    ) -> Result<ParticipantLeftEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;
        let joined_at = Self::timestamp_to_datetime(request.joined_at)?;

        Ok(ParticipantLeftEvent::new(
            request.id,
            user_id,
            *room.id(),
            joined_at,
            created_at,
        ))
    }

    fn map_track_published(
        request: TrackEventRequest,
        room: &Room,
    ) -> Result<TrackPublishedEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let track_type = TrackType::from_string(&request.r#type)?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;

        Ok(TrackPublishedEvent::new(
            request.id,
            user_id,
            *room.id(),
            track_type,
            created_at,
        ))
    }

    fn map_track_unpublished(
        request: TrackEventRequest,
        room: &Room,
    ) -> Result<TrackUnpublishedEvent, EventMapperError> {
        let user_id = UserId::from_string(&request.user_id)?;
        let track_type = TrackType::from_string(&request.r#type)?;
        let created_at = Self::timestamp_to_datetime(request.created_at)?;

        Ok(TrackUnpublishedEvent::new(
            request.id,
            user_id,
            *room.id(),
            track_type,
            created_at,
        ))
    }

    pub fn to_domain_event(dto: RoomEventDto, room: &Room) -> Result<RoomEvent, EventMapperError> {
        match dto {
            RoomEventDto::RoomStarted(request) => Ok(RoomEvent::RoomStarted(
                Self::map_room_started(request, room)?,
            )),
            RoomEventDto::RoomFinished(request) => Ok(RoomEvent::RoomFinished(
                Self::map_room_finished(request, room)?,
            )),
            RoomEventDto::ParticipantJoined(request) => Ok(RoomEvent::ParticipantJoined(
                Self::map_participant_joined(request, room)?,
            )),
            RoomEventDto::ParticipantLeft(request) => Ok(RoomEvent::ParticipantLeft(
                Self::map_participant_left(request, room)?,
            )),
            RoomEventDto::TrackPublished(request) => Ok(RoomEvent::TrackPublished(
                Self::map_track_published(request, room)?,
            )),
            RoomEventDto::TrackUnpublished(request) => Ok(RoomEvent::TrackUnpublished(
                Self::map_track_unpublished(request, room)?,
            )),
        }
    }
}
