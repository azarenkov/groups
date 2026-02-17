use super::common::Room;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomEventRequest {
    pub created_at: u64,
    pub event: String,
    pub id: String,
    pub room: Room,
}
