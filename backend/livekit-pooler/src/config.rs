use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PoolerConfig {
    #[serde(default = "default_interval_secs")]
    pub livekit_pooler_interval_secs: u64,
    #[serde(default = "default_grace_secs")]
    pub livekit_pooler_grace_secs: u64,
    #[serde(default = "default_request_timeout_secs")]
    pub livekit_pooler_request_timeout_secs: u64,
    #[serde(default = "default_max_aborts_per_pass")]
    pub livekit_pooler_max_aborts_per_pass: usize,
    #[serde(default = "default_offline_threshold_passes")]
    pub livekit_pooler_offline_threshold_passes: u32,
    #[serde(default = "default_health_port")]
    pub livekit_pooler_health_port: u16,
}

impl PoolerConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<PoolerConfig>()
    }
}

fn default_interval_secs() -> u64 {
    30
}

fn default_grace_secs() -> u64 {
    20
}

fn default_request_timeout_secs() -> u64 {
    10
}

fn default_max_aborts_per_pass() -> usize {
    500
}

fn default_offline_threshold_passes() -> u32 {
    2
}

fn default_health_port() -> u16 {
    8086
}
