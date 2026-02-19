#[derive(Debug)]
pub struct TrackEventRequest {
    pub created_at: String,
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub room_id: String,
    pub r#type: String,
    pub server_id: String,
}
