#[derive(Debug)]
pub struct ServerDescription(String);

impl ServerDescription {
    pub fn new(_0: String) -> Self {
        Self(_0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
