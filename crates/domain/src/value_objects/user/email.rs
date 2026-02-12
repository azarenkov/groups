#[derive(Debug)]
pub struct Email(String);

impl Email {
    pub fn new(_0: String) -> Self {
        Self(_0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
