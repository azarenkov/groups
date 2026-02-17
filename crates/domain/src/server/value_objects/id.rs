use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ServerId(Uuid);

impl ServerId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
