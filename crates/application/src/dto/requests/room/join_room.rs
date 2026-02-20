use uuid::Uuid;

#[derive(Debug)]
pub struct JoinRoomRequest {
    pub room_id: Uuid,
    pub server_id: Uuid,
}

impl JoinRoomRequest {
    pub fn new(room_id: Uuid, server_id: Uuid) -> Self {
        Self { room_id, server_id }
    }
}
