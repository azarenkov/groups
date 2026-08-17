use std::{num::NonZeroU32, sync::Arc};

use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    state::keyed::DefaultKeyedStateStore,
};
use uuid::Uuid;

use crate::rate_limiting::config::RateLimitingConfig;

pub mod config;

pub type UserRateLimiter =
    RateLimiter<Uuid, DefaultKeyedStateStore<Uuid>, DefaultClock>;

pub fn create_rate_limiter(config: RateLimitingConfig) -> Option<Arc<UserRateLimiter>> {
    create_keyed_limiter(config.requests_per_second, config.burst_size)
}

pub fn create_keyed_limiter(
    requests_per_second: u32,
    burst_size: u32,
) -> Option<Arc<UserRateLimiter>> {
    let requests = NonZeroU32::new(requests_per_second)?;
    let burst = NonZeroU32::new(burst_size)?;
    let quota = Quota::per_second(requests).allow_burst(burst);
    Some(Arc::new(RateLimiter::keyed(quota)))
}
