use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ServerName(String);

impl ServerName {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
