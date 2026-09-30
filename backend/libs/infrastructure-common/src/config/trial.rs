use chrono::Duration as ChronoDuration;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct TrialConfig {
    #[serde(default = "default_ip_limit")]
    pub trial_ip_limit: i64,
    #[serde(default = "default_ip_window_days")]
    pub trial_ip_window_days: i64,
    #[serde(default = "default_expiry_interval_secs")]
    pub entitlements_expiry_interval_secs: u64,
    #[serde(default = "default_expiry_batch_size")]
    pub entitlements_expiry_batch_size: i64,
}

fn default_ip_limit() -> i64 {
    2
}

fn default_ip_window_days() -> i64 {
    30
}

fn default_expiry_interval_secs() -> u64 {
    60
}

fn default_expiry_batch_size() -> i64 {
    200
}

impl TrialConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    pub fn ip_limit(&self) -> i64 {
        self.trial_ip_limit.clamp(1, 1000)
    }

    pub fn ip_window(&self) -> ChronoDuration {
        ChronoDuration::days(self.trial_ip_window_days.clamp(1, 365))
    }

    pub fn expiry_interval(&self) -> Duration {
        Duration::from_secs(self.entitlements_expiry_interval_secs.clamp(10, 3600))
    }

    pub fn expiry_batch_size(&self) -> i64 {
        self.entitlements_expiry_batch_size.clamp(1, 1000)
    }
}
