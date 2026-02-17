#[derive(Debug)]
pub struct RoomEventRequest {
    pub created_at: u64,
    pub id: String,
    pub room_id: String,
}
