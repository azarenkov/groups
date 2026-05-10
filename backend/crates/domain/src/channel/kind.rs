use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub enum ChannelKind {
    Text,
    Voice,
    Hybrid,
}

impl ChannelKind {
    pub fn supports_voice(&self) -> bool {
        matches!(self, Self::Voice | Self::Hybrid)
    }

    pub fn supports_text(&self) -> bool {
        matches!(self, Self::Text | Self::Hybrid)
    }
}
