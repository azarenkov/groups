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

const GCRA_SCRIPT: &str = r#"
local time = redis.call('TIME')
local now = time[1] * 1000 + math.floor(time[2] / 1000)
local emission = tonumber(ARGV[1])
local tolerance = tonumber(ARGV[2])
local tat = tonumber(redis.call('GET', KEYS[1]) or now)
if tat < now then
    tat = now
end
if tat - now > tolerance then
    return 0
end
local new_tat = tat + emission
redis.call('SET', KEYS[1], new_tat, 'PX', (new_tat - now) + tolerance + 1000)
return 1
"#;

pub struct RedisRateLimiter {
    connection: redis::aio::ConnectionManager,
    script: redis::Script,
    scope: String,
    emission_interval_ms: u64,
    delay_tolerance_ms: u64,
}

impl RedisRateLimiter {
    pub async fn connect(
        redis_url: &str,
        scope: &str,
        requests_per_second: u32,
        burst_size: u32,
    ) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let connection = client.get_connection_manager().await?;
        let emission_interval_ms = (1000 / requests_per_second.max(1)).max(1) as u64;
        let delay_tolerance_ms =
            emission_interval_ms * (burst_size.max(1) as u64 - 1);
        Ok(Self {
            connection,
            script: redis::Script::new(GCRA_SCRIPT),
            scope: scope.to_string(),
            emission_interval_ms,
            delay_tolerance_ms,
        })
    }

    async fn check(&self, key: &Uuid) -> bool {
        let mut connection = self.connection.clone();
        let result: Result<i64, redis::RedisError> = self
            .script
            .key(format!("rl:{}:{}", self.scope, key))
            .arg(self.emission_interval_ms)
            .arg(self.delay_tolerance_ms)
            .invoke_async(&mut connection)
            .await;
        match result {
            Ok(allowed) => allowed == 1,
            Err(e) => {
                tracing::warn!(error = %e, scope = %self.scope, "Redis rate limiter unavailable; allowing request");
                true
            }
        }
    }
}

pub enum KeyedRateLimiter {
    InMemory(Arc<UserRateLimiter>),
    Redis(RedisRateLimiter),
}

impl KeyedRateLimiter {
    pub async fn check_key(&self, key: &Uuid) -> bool {
        match self {
            Self::InMemory(limiter) => limiter.check_key(key).is_ok(),
            Self::Redis(limiter) => limiter.check(key).await,
        }
    }
}

pub async fn create_limiter(
    scope: &str,
    requests_per_second: u32,
    burst_size: u32,
) -> Option<KeyedRateLimiter> {
    let redis_url = std::env::var("REDIS_URL").ok().filter(|u| !u.is_empty());
    match redis_url {
        Some(url) => {
            match RedisRateLimiter::connect(&url, scope, requests_per_second, burst_size).await
            {
                Ok(limiter) => {
                    tracing::info!(scope, "Rate limiter backed by Redis");
                    Some(KeyedRateLimiter::Redis(limiter))
                }
                Err(e) => {
                    tracing::warn!(error = %e, scope, "Redis unavailable; falling back to in-memory rate limiter");
                    create_keyed_limiter(requests_per_second, burst_size)
                        .map(KeyedRateLimiter::InMemory)
                }
            }
        }
        None => {
            tracing::warn!(
                scope,
                "REDIS_URL is not set; using in-memory rate limiter (not shared across replicas)"
            );
            create_keyed_limiter(requests_per_second, burst_size).map(KeyedRateLimiter::InMemory)
        }
    }
}

pub async fn create_rate_limiter_from_config(
    scope: &str,
    config: &RateLimitingConfig,
) -> Option<KeyedRateLimiter> {
    create_limiter(scope, config.requests_per_second, config.burst_size).await
}

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
