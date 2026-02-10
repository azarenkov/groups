use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomStatus {
    Active,
    Paused,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomType {
    Public,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomParticipant {
    pub user_id: Uuid,
    pub nickname: String,
    pub is_muted: bool,
    pub is_deafened: bool,
    pub joined_at: DateTime<Utc>,
    pub is_speaking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    pub room_type: RoomType,
    pub status: RoomStatus,
    pub owner_id: Uuid,
    pub max_participants: u32,
    pub participants: HashMap<Uuid, RoomParticipant>,
    pub livekit_room_name: String,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Room {
    pub fn new(name: String, owner_id: Uuid, room_type: RoomType, max_participants: u32) -> Self {
        let id = Uuid::new_v4();
        let now = Utc::now();

        Self {
            id,
            livekit_room_name: format!("room_{}", id),
            name,
            room_type,
            status: RoomStatus::Active,
            owner_id,
            max_participants,
            participants: HashMap::new(),
            password_hash: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_participant(&mut self, participant: RoomParticipant) -> Result<(), RoomError> {
        if self.participants.len() >= self.max_participants as usize {
            return Err(RoomError::RoomFull);
        }

        if self.status != RoomStatus::Active {
            return Err(RoomError::RoomNotActive);
        }

        self.participants.insert(participant.user_id, participant);
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn remove_participant(&mut self, user_id: Uuid) -> Result<(), RoomError> {
        self.participants
            .remove(&user_id)
            .ok_or(RoomError::ParticipantNotFound)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn toggle_participant_mute(&mut self, user_id: Uuid) -> Result<(), RoomError> {
        let participant = self
            .participants
            .get_mut(&user_id)
            .ok_or(RoomError::ParticipantNotFound)?;

        participant.is_muted = !participant.is_muted;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_speaking_status(
        &mut self,
        user_id: Uuid,
        is_speaking: bool,
    ) -> Result<(), RoomError> {
        let participant = self
            .participants
            .get_mut(&user_id)
            .ok_or(RoomError::ParticipantNotFound)?;

        participant.is_speaking = is_speaking;
        Ok(())
    }

    pub fn is_owner(&self, user_id: Uuid) -> bool {
        self.owner_id == user_id
    }

    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    pub fn close(&mut self) {
        self.status = RoomStatus::Closed;
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RoomError {
    #[error("Room is full")]
    RoomFull,

    #[error("Room isn't active")]
    RoomNotActive,

    #[error("User not found")]
    ParticipantNotFound,

    #[error("Insufficient ermissions")]
    InsufficientPermissions,
}
