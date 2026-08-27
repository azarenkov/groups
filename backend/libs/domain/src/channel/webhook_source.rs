#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookSource {
    LiveKit,
    LiveKitPooler,
}

impl WebhookSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LiveKit => "livekit",
            Self::LiveKitPooler => "livekit_pooler",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "livekit" => Some(Self::LiveKit),
            "livekit_pooler" => Some(Self::LiveKitPooler),
            _ => None,
        }
    }
}
