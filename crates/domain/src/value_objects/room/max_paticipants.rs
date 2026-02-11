use serde::{Deserialize, Serialize};

const MAX_PARTICIPANTS: u8 = 20;

#[derive(Debug, Serialize, Deserialize)]
pub struct MaxParticipants(u8);

impl MaxParticipants {
    pub fn new(_0: u8) -> Self {
        Self(_0)
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl Default for MaxParticipants {
    fn default() -> Self {
        Self(MAX_PARTICIPANTS)
    }
}
