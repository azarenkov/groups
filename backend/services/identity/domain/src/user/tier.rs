use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tier {
    #[default]
    Free,
    Pro,
    Premium,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Premium => "premium",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "free" => Some(Self::Free),
            "pro" => Some(Self::Pro),
            "premium" => Some(Self::Premium),
            _ => None,
        }
    }

    pub fn is_paid(&self) -> bool {
        !matches!(self, Self::Free)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Entitlements {
    pub tier: Tier,
    pub valid_until: Option<DateTime<Utc>>,
}

impl Default for Entitlements {
    fn default() -> Self {
        Self {
            tier: Tier::Free,
            valid_until: None,
        }
    }
}

impl Entitlements {
    pub fn active_at(&self, now: DateTime<Utc>) -> Tier {
        match self.valid_until {
            Some(until) if until <= now => Tier::Free,
            _ => self.tier,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub enum BadgeStyle {
    #[default]
    Default,
    Tint,
    Solid,
    Neutral,
    Outline,
    Ink,
    Etched,
    Numbered,
}

impl BadgeStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "",
            Self::Tint => "tint",
            Self::Solid => "solid",
            Self::Neutral => "neutral",
            Self::Outline => "outline",
            Self::Ink => "ink",
            Self::Etched => "etched",
            Self::Numbered => "numbered",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "" => Some(Self::Default),
            "tint" => Some(Self::Tint),
            "solid" => Some(Self::Solid),
            "neutral" => Some(Self::Neutral),
            "outline" => Some(Self::Outline),
            "ink" => Some(Self::Ink),
            "etched" => Some(Self::Etched),
            "numbered" => Some(Self::Numbered),
            _ => None,
        }
    }

    pub fn needs_paid_tier(&self) -> bool {
        !matches!(self, Self::Default)
    }
}

#[async_trait::async_trait]
pub trait EntitlementsRepository: Send + Sync {
    async fn get(
        &self,
        user_id: &domain::user::id::UserId,
    ) -> Result<Entitlements, ports::repository::RepositoryError>;
}
