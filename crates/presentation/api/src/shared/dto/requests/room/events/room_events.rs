use super::common::Room;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomEventRequest {
    pub created_at: String,
    pub id: String,
    pub room: Room,
}
