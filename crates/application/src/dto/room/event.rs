#[derive(Debug)]
pub enum RoomEvent {
    ParticipantJoined {
        room: RoomInfo,
        participant: ParticipantInfo,
    },
    ParticipantLeft {
        room: RoomInfo,
        participant: ParticipantInfo,
    },
}

#[derive(Debug)]
pub struct RoomInfo {
    pub sid: String,
    pub name: String,
    pub num_participants: u32,
    pub max_participants: Option<u32>,
}

#[derive(Debug)]
pub struct ParticipantInfo {
    pub sid: String,
    pub identity: String,
    pub name: String,
    pub state: Option<String>,
    pub metadata: Option<String>,
    pub joined_at: Option<i64>,
}

#[derive(Debug)]
pub struct RoomEventNotification {
    pub event_type: RoomEventType,
    pub room_name: String,
    pub participant: ParticipantInfo,
}

#[derive(Debug)]
pub enum RoomEventType {
    ParticipantJoined,
    ParticipantLeft,
}
