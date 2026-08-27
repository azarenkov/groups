use uuid::Uuid;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RefreshTokenId(Uuid);

#[derive(Debug, thiserror::Error)]
#[error("Invalid refresh token ID format")]
pub struct InvalidRefreshTokenId;

impl RefreshTokenId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, InvalidRefreshTokenId> {
        let uuid = Uuid::parse_str(s).map_err(|_| InvalidRefreshTokenId)?;
        Ok(Self(uuid))
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
