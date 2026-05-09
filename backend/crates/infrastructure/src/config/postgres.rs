use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PostgresConfig {
    #[serde(rename = "postgres_url")]
    url: String,
    #[serde(rename = "postgres_max_connections")]
    max_connections: u32,
    #[serde(rename = "postgres_min_connections")]
    min_connections: u32,
    #[serde(rename = "postgres_connect_timeout_seconds")]
    connect_timeout_seconds: u64,
    #[serde(rename = "postgres_idle_timeout_seconds")]
    idle_timeout_seconds: u64,
    #[serde(rename = "postgres_max_lifetime_seconds")]
    max_lifetime_seconds: u64,
}

impl PostgresConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<PostgresConfig>()
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }

    pub fn min_connections(&self) -> u32 {
        self.min_connections
    }

    pub fn connect_timeout_seconds(&self) -> u64 {
        self.connect_timeout_seconds
    }

    pub fn idle_timeout_seconds(&self) -> u64 {
        self.idle_timeout_seconds
    }

    pub fn max_lifetime_seconds(&self) -> u64 {
        self.max_lifetime_seconds
    }
}
