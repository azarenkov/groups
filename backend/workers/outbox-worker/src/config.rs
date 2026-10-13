use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WorkerConfig {
    #[serde(default = "default_sources")]
    pub outbox_sources: String,
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
    #[serde(default = "default_liveness_stale_secs")]
    pub outbox_liveness_stale_secs: u64,
    #[serde(default = "default_metrics_interval_secs")]
    pub outbox_metrics_interval_secs: u64,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<WorkerConfig>()
    }

    pub fn sources(&self) -> Result<Vec<String>, String> {
        parse_sources(&self.outbox_sources)
    }
}

pub fn parse_sources(raw: &str) -> Result<Vec<String>, String> {
    let mut sources: Vec<String> = Vec::new();
    for source in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if !infrastructure_common::database::postgres::connection::is_valid_schema(source) {
            return Err(format!("invalid outbox source {source:?}"));
        }
        if !sources.iter().any(|s| s == source) {
            sources.push(source.to_string());
        }
    }
    if sources.is_empty() {
        return Err("OUTBOX_SOURCES lists no databases".to_string());
    }
    Ok(sources)
}

fn default_sources() -> String {
    "identity,messaging,voice".to_string()
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

fn default_liveness_stale_secs() -> u64 {
    60
}

fn default_metrics_interval_secs() -> u64 {
    15
}

#[cfg(test)]
mod tests {
    use super::parse_sources;

    #[test]
    fn parses_trimmed_unique_sources() {
        assert_eq!(
            parse_sources(" identity, messaging ,voice,messaging,").unwrap(),
            vec!["identity", "messaging", "voice"],
        );
    }

    #[test]
    fn rejects_empty_or_unsafe_sources() {
        assert!(parse_sources(" , ").is_err());
        assert!(parse_sources("identity,public;drop").is_err());
    }
}
