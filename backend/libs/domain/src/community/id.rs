use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct CommunityId(Uuid);

#[derive(Debug, thiserror::Error)]
#[error("Invalid community ID format")]
pub struct InvalidCommunityId;

impl CommunityId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, InvalidCommunityId> {
        let uuid = Uuid::parse_str(s).map_err(|_| InvalidCommunityId)?;
        Ok(Self(uuid))
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
