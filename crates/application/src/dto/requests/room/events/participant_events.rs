#[derive(Debug)]
pub struct ParticipantEventRequest {
    pub created_at: String,
    pub id: String,
    pub user_id: String,
    pub joined_at: String,
    pub room_id: String,
}
