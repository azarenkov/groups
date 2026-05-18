use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DmMessageId(Uuid);

impl DmMessageId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn uuid(&self) -> Uuid {
        self.0
    }
}
