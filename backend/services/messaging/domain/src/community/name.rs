use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CommunityName(String);

impl CommunityName {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
