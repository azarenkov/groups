use uuid::Uuid;

#[derive(Debug)]
pub struct RoomId(Uuid);

impl RoomId {
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
