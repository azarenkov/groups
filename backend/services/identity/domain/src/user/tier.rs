use chrono::{DateTime, Duration, Utc};

pub const TRIAL_DAYS: i64 = 7;

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

    fn rank(&self) -> u8 {
        match self {
            Self::Free => 0,
            Self::Pro => 1,
            Self::Premium => 2,
        }
    }

    pub fn includes(&self, required: Tier) -> bool {
        self.rank() >= required.rank()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrialHistory {
    pub email_used: bool,
    pub recent_from_ip: i64,
}

pub fn trial_email_key(email: &str) -> String {
    let email = email.trim().to_lowercase();
    match email.split_once('@') {
        Some((local, domain)) => {
            let local = local.split('+').next().unwrap_or(local);
            let local = if matches!(domain, "gmail.com" | "googlemail.com") {
                local.replace('.', "")
            } else {
                local.to_string()
            };
            let domain = if domain == "googlemail.com" { "gmail.com" } else { domain };
            format!("{local}@{domain}")
        }
        None => email,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Entitlements {
    pub tier: Tier,
    pub valid_until: Option<DateTime<Utc>>,
    pub member_number: Option<i64>,
    pub trial: bool,
    pub trial_used_at: Option<DateTime<Utc>>,
}

impl Default for Entitlements {
    fn default() -> Self {
        Self {
            tier: Tier::Free,
            valid_until: None,
            member_number: None,
            trial: false,
            trial_used_at: None,
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

    pub fn on_trial_at(&self, now: DateTime<Utc>) -> bool {
        self.trial && self.active_at(now).is_paid()
    }

    pub fn trial_available_at(&self, now: DateTime<Utc>) -> bool {
        self.trial_used_at.is_none() && !self.active_at(now).is_paid()
    }

    pub fn trial_ends_at(now: DateTime<Utc>) -> DateTime<Utc> {
        now + Duration::days(TRIAL_DAYS)
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
    NumberOnly,
    Gold,
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
            Self::NumberOnly => "number_only",
            Self::Gold => "gold",
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
            "number_only" => Some(Self::NumberOnly),
            "gold" => Some(Self::Gold),
            _ => None,
        }
    }

    pub fn needs_paid_tier(&self) -> bool {
        !matches!(self, Self::Default)
    }

    pub fn required_tier(&self) -> Tier {
        match self {
            Self::Default => Tier::Free,
            Self::Gold => Tier::Premium,
            _ => Tier::Pro,
        }
    }
}

#[async_trait::async_trait]
pub trait EntitlementsRepository: Send + Sync {
    async fn get(
        &self,
        user_id: &domain::user::id::UserId,
    ) -> Result<Entitlements, ports::repository::RepositoryError>;
}

#[cfg(test)]
mod badge_style_tests {
    use super::BadgeStyle;

    #[test]
    fn every_style_round_trips_through_its_name() {
        for style in [
            BadgeStyle::Default,
            BadgeStyle::Tint,
            BadgeStyle::Solid,
            BadgeStyle::Neutral,
            BadgeStyle::Outline,
            BadgeStyle::Ink,
            BadgeStyle::Etched,
            BadgeStyle::Numbered,
            BadgeStyle::NumberOnly,
            BadgeStyle::Gold,
        ] {
            assert_eq!(BadgeStyle::parse(style.as_str()), Some(style));
        }
    }

    #[test]
    fn gold_is_reserved_for_premium() {
        use super::Tier;
        assert_eq!(BadgeStyle::parse("gold"), Some(BadgeStyle::Gold));
        assert_eq!(BadgeStyle::Gold.required_tier(), Tier::Premium);
        assert_eq!(BadgeStyle::Ink.required_tier(), Tier::Pro);
        assert_eq!(BadgeStyle::Default.required_tier(), Tier::Free);
        assert!(Tier::Premium.includes(Tier::Pro));
        assert!(!Tier::Pro.includes(Tier::Premium));
        assert!(Tier::Free.includes(Tier::Free));
    }

    #[test]
    fn number_only_is_a_paid_style_and_unknown_names_are_rejected() {
        assert_eq!(BadgeStyle::parse("number_only"), Some(BadgeStyle::NumberOnly));
        assert!(BadgeStyle::NumberOnly.needs_paid_tier());
        assert_eq!(BadgeStyle::parse("sparkly"), None);
    }
}
