use chrono::Duration as ChronoDuration;
use serde::Deserialize;
use std::time::Duration;

const MIN_TTL_HOURS: i64 = 1;
const MIN_INTERVAL_SECS: u64 = 60;
const MIN_BATCH_SIZE: i64 = 1;
const MAX_BATCH_SIZE: i64 = 1000;

#[derive(Debug, Clone, Deserialize)]
pub struct UnverifiedCleanupConfig {
    #[serde(default = "default_enabled")]
    pub unverified_cleanup_enabled: bool,
    #[serde(default = "default_ttl_hours")]
    pub unverified_account_ttl_hours: i64,
    #[serde(default = "default_interval_secs")]
    pub unverified_cleanup_interval_secs: u64,
    #[serde(default = "default_batch_size")]
    pub unverified_cleanup_batch_size: i64,
}

fn default_enabled() -> bool {
    true
}

fn default_ttl_hours() -> i64 {
    24
}

fn default_interval_secs() -> u64 {
    900
}

fn default_batch_size() -> i64 {
    200
}

impl UnverifiedCleanupConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    pub fn enabled(&self) -> bool {
        self.unverified_cleanup_enabled
    }

    pub fn ttl(&self) -> ChronoDuration {
        ChronoDuration::hours(self.unverified_account_ttl_hours.max(MIN_TTL_HOURS))
    }

    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.unverified_cleanup_interval_secs.max(MIN_INTERVAL_SECS))
    }

    pub fn batch_size(&self) -> i64 {
        self.unverified_cleanup_batch_size
            .clamp(MIN_BATCH_SIZE, MAX_BATCH_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::UnverifiedCleanupConfig;

    fn config(ttl_hours: i64, interval_secs: u64, batch_size: i64) -> UnverifiedCleanupConfig {
        UnverifiedCleanupConfig {
            unverified_cleanup_enabled: true,
            unverified_account_ttl_hours: ttl_hours,
            unverified_cleanup_interval_secs: interval_secs,
            unverified_cleanup_batch_size: batch_size,
        }
    }

    #[test]
    fn defaults_keep_accounts_for_a_day() {
        let parsed: UnverifiedCleanupConfig =
            envy::from_iter(Vec::<(String, String)>::new()).unwrap();
        assert!(parsed.enabled());
        assert_eq!(parsed.ttl().num_hours(), 24);
        assert_eq!(parsed.interval().as_secs(), 900);
        assert_eq!(parsed.batch_size(), 200);
    }

    #[test]
    fn a_zero_lifetime_never_deletes_fresh_accounts() {
        assert_eq!(config(0, 900, 200).ttl().num_hours(), 1);
        assert_eq!(config(-5, 900, 200).ttl().num_hours(), 1);
    }

    #[test]
    fn sweeps_stay_within_safe_bounds() {
        assert_eq!(config(24, 1, 200).interval().as_secs(), 60);
        assert_eq!(config(24, 900, 0).batch_size(), 1);
        assert_eq!(config(24, 900, 50_000).batch_size(), 1000);
    }
}
