pub mod repository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationScope {
    Channel,
    Dm,
}

impl NotificationScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationScope::Channel => "channel",
            NotificationScope::Dm => "dm",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "channel" => Some(NotificationScope::Channel),
            "dm" => Some(NotificationScope::Dm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    All,
    Mentions,
    Muted,
}

impl NotificationLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationLevel::All => "all",
            NotificationLevel::Mentions => "mentions",
            NotificationLevel::Muted => "none",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "all" => Some(NotificationLevel::All),
            "mentions" => Some(NotificationLevel::Mentions),
            "none" => Some(NotificationLevel::Muted),
            _ => None,
        }
    }
}
