use serde::Deserialize;
use std::time::Duration;

const MIN_INTERVAL_SECS: u64 = 5;
const MAX_INTERVAL_SECS: u64 = 60 * 60;
const MIN_BATCH_SIZE: i64 = 1;
const MAX_BATCH_SIZE: i64 = 1000;

#[derive(Debug, Clone, Deserialize)]
pub struct AccessRevocationRetryConfig {
    #[serde(default = "default_interval_secs")]
    pub access_revocation_retry_interval_secs: u64,
    #[serde(default = "default_batch_size")]
    pub access_revocation_retry_batch_size: i64,
}

fn default_interval_secs() -> u64 {
    30
}

fn default_batch_size() -> i64 {
    100
}

impl AccessRevocationRetryConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    pub fn interval(&self) -> Duration {
        Duration::from_secs(
            self.access_revocation_retry_interval_secs
                .clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS),
        )
    }

    pub fn batch_size(&self) -> i64 {
        self.access_revocation_retry_batch_size
            .clamp(MIN_BATCH_SIZE, MAX_BATCH_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::AccessRevocationRetryConfig;

    #[test]
    fn defaults_retry_every_half_minute() {
        let parsed: AccessRevocationRetryConfig = envy::from_iter(Vec::<(String, String)>::new()).unwrap();
        assert_eq!(parsed.interval().as_secs(), 30);
        assert_eq!(parsed.batch_size(), 100);
    }

    #[test]
    fn retries_stay_within_safe_bounds() {
        let config = AccessRevocationRetryConfig {
            access_revocation_retry_interval_secs: 0,
            access_revocation_retry_batch_size: 50_000,
        };
        assert_eq!(config.interval().as_secs(), 5);
        assert_eq!(config.batch_size(), 1000);
    }
}
