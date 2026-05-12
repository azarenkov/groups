use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VerificationCodeId(Uuid);

#[derive(Debug, thiserror::Error)]
#[error("Invalid verification code ID format")]
pub struct InvalidVerificationCodeId;

impl VerificationCodeId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, InvalidVerificationCodeId> {
        let uuid = Uuid::parse_str(s).map_err(|_| InvalidVerificationCodeId)?;
        Ok(Self(uuid))
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
