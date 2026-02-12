#[derive(Debug)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn new(_0: String) -> Self {
        Self(_0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
