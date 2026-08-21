use serde::Serialize;

pub const USERNAME_MIN_LEN: usize = 3;
pub const USERNAME_MAX_LEN: usize = 15;

const DEFAULT_RESERVED: &[&str] = &[
    "admin",
    "administrator",
    "moderator",
    "mod",
    "system",
    "rift",
    "support",
    "help",
    "root",
    "owner",
    "staff",
    "everyone",
    "here",
    "me",
    "null",
    "undefined",
    "deleted",
    "unknown",
];

#[derive(Debug, thiserror::Error)]
pub enum UserNameError {
    #[error("Username must be at least {USERNAME_MIN_LEN} characters")]
    TooShort,
    #[error("Username must be at most {USERNAME_MAX_LEN} characters")]
    TooLong,
    #[error("Username may only contain latin letters, digits and underscores")]
    InvalidCharacters,
    #[error("Username cannot start or end with an underscore")]
    UnderscoreEdge,
    #[error("Username cannot contain consecutive underscores")]
    DoubleUnderscore,
    #[error("Username cannot consist of digits only")]
    DigitsOnly,
    #[error("Username is reserved")]
    Reserved,
}

#[derive(Debug, Clone, Default)]
pub struct ReservedNames {
    extra: Vec<String>,
}

impl ReservedNames {
    pub fn new(extra: Vec<String>) -> Self {
        Self {
            extra: extra
                .into_iter()
                .map(|n| n.trim().to_ascii_lowercase())
                .filter(|n| !n.is_empty())
                .collect(),
        }
    }

    pub fn contains(&self, lower_name: &str) -> bool {
        DEFAULT_RESERVED.contains(&lower_name)
            || self.extra.iter().any(|n| n == lower_name)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UserName(String);

impl UserName {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn parse(value: &str, reserved: &ReservedNames) -> Result<Self, UserNameError> {
        let value = value.trim();
        if value.len() < USERNAME_MIN_LEN {
            return Err(UserNameError::TooShort);
        }
        if value.len() > USERNAME_MAX_LEN {
            return Err(UserNameError::TooLong);
        }
        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(UserNameError::InvalidCharacters);
        }
        if value.starts_with('_') || value.ends_with('_') {
            return Err(UserNameError::UnderscoreEdge);
        }
        if value.contains("__") {
            return Err(UserNameError::DoubleUnderscore);
        }
        if value.chars().all(|c| c.is_ascii_digit()) {
            return Err(UserNameError::DigitsOnly);
        }
        if reserved.contains(&value.to_ascii_lowercase()) {
            return Err(UserNameError::Reserved);
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
