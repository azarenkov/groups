use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
pub struct MessageId(Uuid);

#[derive(Debug, thiserror::Error)]
#[error("Invalid message ID format")]
pub struct InvalidMessageId;

impl MessageId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, InvalidMessageId> {
        let uuid = Uuid::parse_str(s).map_err(|_| InvalidMessageId)?;
        Ok(Self(uuid))
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
