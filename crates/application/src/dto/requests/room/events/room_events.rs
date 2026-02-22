#[derive(Debug)]
pub struct RoomEventRequest {
    pub created_at: String,
    pub id: String,
    pub room_id: String,
    pub server_id: Option<String>,
}
