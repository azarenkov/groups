use application::dto::requests::room::events::{
    RoomEventDto, participant_events::ParticipantEventRequest as AppParticipantEventRequest,
    room_events::RoomEventRequest as AppRoomEventRequest,
    track_events::TrackEventRequest as AppTrackEventRequest,
};

use crate::shared::dto::requests::room::events::{
    participant_events::ParticipantEventRequest as PresentationParticipantEventRequest,
    room_events::RoomEventRequest as PresentationRoomEventRequest,
    track_events::TrackEventRequest as PresentationTrackEventRequest,
};

pub struct RoomEventMapper;

impl RoomEventMapper {
    fn map_room_event(request: PresentationRoomEventRequest) -> AppRoomEventRequest {
        AppRoomEventRequest {
            created_at: request.created_at,
            id: request.id,
            room_id: request.room.name,
        }
    }

    fn map_participant_event(
        request: PresentationParticipantEventRequest,
    ) -> Result<AppParticipantEventRequest, MapperError> {
        let joined_at = request
            .participant
            .joined_at
            .ok_or(MapperError::MissingJoinedAt)?;

        Ok(AppParticipantEventRequest {
            created_at: request.created_at,
            id: request.id,
            user_id: request.participant.identity,
            joined_at,
            room_id: request.room.name,
        })
    }

    fn map_track_event(request: PresentationTrackEventRequest) -> AppTrackEventRequest {
        AppTrackEventRequest {
            created_at: request.created_at,
            id: request.id,
            user_id: request.participant.identity,
            room_id: request.room.name,
            r#type: request.track.r#type,
        }
    }

    pub fn to_application_dto(
        event: String,
        data: serde_json::Value,
    ) -> Result<RoomEventDto, MapperError> {
        match event.as_str() {
            "room_started" => {
                let request: PresentationRoomEventRequest = serde_json::from_value(data)?;
                Ok(RoomEventDto::RoomStarted(Self::map_room_event(request)))
            }
            "room_finished" => {
                let request: PresentationRoomEventRequest = serde_json::from_value(data)?;
                Ok(RoomEventDto::RoomFinished(Self::map_room_event(request)))
            }
            "participant_joined" => {
                let request: PresentationParticipantEventRequest = serde_json::from_value(data)?;
                Ok(RoomEventDto::ParticipantJoined(
                    Self::map_participant_event(request)?,
                ))
            }
            "participant_left" => {
                let request: PresentationParticipantEventRequest = serde_json::from_value(data)?;
                Ok(RoomEventDto::ParticipantLeft(Self::map_participant_event(
                    request,
                )?))
            }
            "track_published" => {
                let request: PresentationTrackEventRequest = serde_json::from_value(data)?;
                Ok(RoomEventDto::TrackPublished(Self::map_track_event(request)))
            }
            "track_unpublished" => {
                let request: PresentationTrackEventRequest = serde_json::from_value(data)?;
                Ok(RoomEventDto::TrackUnpublished(Self::map_track_event(
                    request,
                )))
            }
            _ => Err(MapperError::UnknownEvent(event)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MapperError {
    #[error("Unknown event type: {0}")]
    UnknownEvent(String),
    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Missing joined_at timestamp for participant event")]
    MissingJoinedAt,
}
