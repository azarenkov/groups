use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChannelName(String);

impl ChannelName {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
