use uuid::Uuid;

#[derive(Debug)]
pub struct GenerateTokenRequest {
    pub room_id: Uuid,
    pub server_id: Uuid,
}
