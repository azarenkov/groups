use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
