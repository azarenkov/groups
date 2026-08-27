use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct ChannelId(Uuid);

#[derive(Debug, thiserror::Error)]
#[error("Invalid channel ID format")]
pub struct InvalidChannelId;

impl ChannelId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, InvalidChannelId> {
        let uuid = Uuid::parse_str(s).map_err(|_| InvalidChannelId)?;
        Ok(Self(uuid))
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
