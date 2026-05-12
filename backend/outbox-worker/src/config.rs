use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WorkerConfig {
    #[serde(default = "default_batch_size")]
    pub outbox_batch_size: u32,
    #[serde(default = "default_idle_interval_ms")]
    pub outbox_idle_interval_ms: u64,
    #[serde(default = "default_cleanup_interval_secs")]
    pub outbox_cleanup_interval_secs: u64,
    #[serde(default = "default_cleanup_retention_secs")]
    pub outbox_cleanup_retention_secs: u64,
    #[serde(default = "default_cleanup_batch_size")]
    pub outbox_cleanup_batch_size: i64,
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

fn default_cleanup_interval_secs() -> u64 {
    60 * 60
}

fn default_cleanup_retention_secs() -> u64 {
    60 * 60 * 24 * 7
}

fn default_cleanup_batch_size() -> i64 {
    10_000
}
