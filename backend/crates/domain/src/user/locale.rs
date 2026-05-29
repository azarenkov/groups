use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    En,
    Ru,
}

impl Locale {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ru => "ru",
        }
    }

    pub fn from_str_lenient(input: &str) -> Self {
        let lower = input.trim().to_ascii_lowercase();
        let prefix = lower.split(['-', '_']).next().unwrap_or("");
        match prefix {
            "ru" => Self::Ru,
            _ => Self::En,
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid locale: {0}")]
pub struct LocaleParseError(pub String);

impl std::str::FromStr for Locale {
    type Err = LocaleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en" => Ok(Self::En),
            "ru" => Ok(Self::Ru),
            other => Err(LocaleParseError(other.to_string())),
        }
    }
}
