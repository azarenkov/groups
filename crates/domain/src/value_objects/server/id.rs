use uuid::Uuid;

#[derive(Debug)]
pub struct ServerId(Uuid);

impl ServerId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
