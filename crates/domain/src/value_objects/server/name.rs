#[derive(Debug)]
pub struct ServerName(String);

impl ServerName {
    pub fn new(_0: String) -> Self {
        Self(_0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
