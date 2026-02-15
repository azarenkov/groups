#[derive(Debug)]
pub struct ServerDescription(String);

impl ServerDescription {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
