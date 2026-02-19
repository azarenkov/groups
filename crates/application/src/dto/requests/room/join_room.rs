use uuid::Uuid;

#[derive(Debug)]
pub struct JoinRoomRequest {
    pub room_id: Uuid,
    pub server_id: Uuid,
}
