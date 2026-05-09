#[derive(Debug)]
pub struct CodeHash(String);

impl CodeHash {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
