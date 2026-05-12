use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WorkerConfig {
    #[serde(default = "default_batch_size")]
    pub outbox_batch_size: u32,
    #[serde(default = "default_idle_interval_ms")]
    pub outbox_idle_interval_ms: u64,
    #[serde(default = "default_max_attempts")]
    pub outbox_max_attempts: i16,
    #[serde(default = "default_backoff_base_secs")]
    pub outbox_backoff_base_secs: i32,
    #[serde(default = "default_backoff_max_multiplier")]
    pub outbox_backoff_max_multiplier: i32,
    #[serde(default = "default_cleanup_interval_secs")]
    pub outbox_cleanup_interval_secs: u64,
    #[serde(default = "default_cleanup_retention_secs")]
    pub outbox_cleanup_retention_secs: u64,
    #[serde(default = "default_cleanup_batch_size")]
    pub outbox_cleanup_batch_size: i64,
    #[serde(default = "default_health_port")]
    pub health_port: u16,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<WorkerConfig>()
    }
}

fn default_batch_size() -> u32 {
    100
}

fn default_idle_interval_ms() -> u64 {
    500
}

fn default_max_attempts() -> i16 {
    50
}

fn default_backoff_base_secs() -> i32 {
    5
}

fn default_backoff_max_multiplier() -> i32 {
    10
}

fn default_cleanup_interval_secs() -> u64 {
    60 * 60
}

fn default_cleanup_retention_secs() -> u64 {
    60 * 60 * 24 * 7
}

fn default_cleanup_batch_size() -> i64 {
    10_000
}

fn default_health_port() -> u16 {
    8081
}
