const MAX_PARTICIPANTS: u8 = 20;

#[derive(Debug)]
pub struct MaxParticipants(u8);

impl MaxParticipants {
    pub fn new(value: u8) -> Self {
        Self(value)
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
