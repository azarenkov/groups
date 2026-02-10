pub struct RoomToken {
    value: String,
}

impl RoomToken {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}
