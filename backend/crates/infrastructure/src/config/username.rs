use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct UsernameConfig {
    #[serde(rename = "reserved_usernames", default)]
    pub reserved_usernames: Option<String>,
}

impl UsernameConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    pub fn extra_reserved(&self) -> Vec<String> {
        self.reserved_usernames
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
