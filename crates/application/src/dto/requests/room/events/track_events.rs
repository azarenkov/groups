#[derive(Debug)]
pub struct TrackEventRequest {
    pub created_at: String,
    pub id: String,
    pub user_id: String,
    pub user_name: Option<String>,
    pub room_id: String,
    pub r#type: Option<String>,
    pub server_id: Option<String>,
}
