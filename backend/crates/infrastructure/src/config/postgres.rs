use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PostgresConfig {
    #[serde(rename = "postgres_host")]
    host: String,
    #[serde(rename = "postgres_port")]
    port: u16,
    #[serde(rename = "postgres_user")]
    user: String,
    #[serde(rename = "postgres_password")]
    password: String,
    #[serde(rename = "postgres_db")]
    db: String,
    #[serde(rename = "postgres_options", default)]
    options: String,
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

    pub fn url(&self) -> String {
        let user = urlencoding::encode(&self.user);
        let password = urlencoding::encode(&self.password);
        let base = format!(
            "postgres://{}:{}@{}:{}/{}",
            user, password, self.host, self.port, self.db,
        );
        if self.options.is_empty() {
            base
        } else {
            format!("{}?{}", base, self.options)
        }
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
