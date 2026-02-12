#[derive(Debug)]
pub struct UserName(String);

impl UserName {
    pub fn new(_0: String) -> Self {
        Self(_0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
