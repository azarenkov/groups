pub struct RoomToken {
    value: String,
}

impl RoomToken {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn to_string(self) -> String {
        self.value
    }
}
