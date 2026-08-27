use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Text,
    Voice,
}

impl ChannelKind {
    pub fn supports_voice(&self) -> bool {
        matches!(self, Self::Voice)
    }

    pub fn supports_text(&self) -> bool {
        matches!(self, Self::Text)
    }
}
