use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct RoomId(Uuid);

#[derive(Debug, thiserror::Error)]
#[error("Invalid room ID format")]
pub struct InvalidRoomId;

impl RoomId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, InvalidRoomId> {
        let uuid = Uuid::parse_str(s).map_err(|_| InvalidRoomId)?;
        Ok(Self(uuid))
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
