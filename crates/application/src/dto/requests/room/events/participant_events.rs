#[derive(Debug)]
pub struct ParticipantEventRequest {
    pub created_at: u64,
    pub id: String,
    pub user_id: String,
    pub joined_at: u64,
    pub room_id: String,
}
