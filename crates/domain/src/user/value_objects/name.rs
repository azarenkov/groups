#[derive(Debug)]
pub struct UserName(String);

impl UserName {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
