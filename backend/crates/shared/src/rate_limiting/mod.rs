use actix_governor::{
    Governor, GovernorConfig, GovernorConfigBuilder,
    governor::{clock::QuantaInstant, middleware::NoOpMiddleware},
};

use crate::rate_limiting::{config::RateLimitingConfig, key_extractor::UserOrIpKeyExtractor};

pub mod config;
pub mod key_extractor;

pub fn create_governor_config(
    config: RateLimitingConfig,
) -> Option<GovernorConfig<UserOrIpKeyExtractor, NoOpMiddleware<QuantaInstant>>> {
    GovernorConfigBuilder::default()
        .requests_per_second(config.requests_per_second)
        .burst_size(config.burst_size)
        .key_extractor(UserOrIpKeyExtractor)
        .finish()
}

pub fn create_rate_limiting(
    governor_config: &GovernorConfig<UserOrIpKeyExtractor, NoOpMiddleware<QuantaInstant>>,
) -> Governor<UserOrIpKeyExtractor, NoOpMiddleware<QuantaInstant>> {
    Governor::new(governor_config)
}
