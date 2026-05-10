use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Clone, Copy)]
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
