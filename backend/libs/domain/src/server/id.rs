use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct ServerId(Uuid);

#[derive(Debug, thiserror::Error)]
#[error("Invalid server ID format")]
pub struct InvalidServerId;

impl ServerId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, InvalidServerId> {
        let uuid = Uuid::parse_str(s).map_err(|_| InvalidServerId)?;
        Ok(Self(uuid))
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
