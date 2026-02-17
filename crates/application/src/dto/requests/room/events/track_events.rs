#[derive(Debug)]
pub struct TrackEventRequest {
    pub created_at: u64,
    pub id: String,
    pub user_id: String,
    pub room_id: String,
    pub r#type: String,
}
