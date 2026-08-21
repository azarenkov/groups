use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NatsConfig {
    #[serde(rename = "nats_url")]
    url: String,
    #[serde(rename = "nats_subject_prefix")]
    pub subject_prefix: String,
    #[serde(rename = "nats_replicas")]
    pub replicas: usize,
    #[serde(rename = "nats_stream_hot_max_bytes", default = "default_hot_max_bytes")]
    pub stream_hot_max_bytes: i64,
    #[serde(rename = "nats_stream_cold_max_bytes", default = "default_cold_max_bytes")]
    pub stream_cold_max_bytes: i64,
    #[serde(rename = "event_partitions", default = "default_event_partitions")]
    pub event_partitions: u32,
}

impl NatsConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<NatsConfig>()
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

fn default_event_partitions() -> u32 {
    16
}

fn default_hot_max_bytes() -> i64 {
    50 * 1024 * 1024 * 1024
}

fn default_cold_max_bytes() -> i64 {
    10 * 1024 * 1024 * 1024
}
