use std::{
    num::NonZeroU32,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

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

const RECONNECT_EVERY: Duration = Duration::from_secs(5);

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

#[derive(Debug, Default)]
pub struct RateLimiterStats {
    redis_checks: AtomicU64,
    redis_errors: AtomicU64,
    local_fallbacks: AtomicU64,
    reconnects: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimiterSnapshot {
    pub redis_checks: u64,
    pub redis_errors: u64,
    pub local_fallbacks: u64,
    pub reconnects: u64,
}

impl RateLimiterStats {
    pub fn snapshot(&self) -> RateLimiterSnapshot {
        RateLimiterSnapshot {
            redis_checks: self.redis_checks.load(Ordering::Relaxed),
            redis_errors: self.redis_errors.load(Ordering::Relaxed),
            local_fallbacks: self.local_fallbacks.load(Ordering::Relaxed),
            reconnects: self.reconnects.load(Ordering::Relaxed),
        }
    }
}

static STATS: RateLimiterStats = RateLimiterStats {
    redis_checks: AtomicU64::new(0),
    redis_errors: AtomicU64::new(0),
    local_fallbacks: AtomicU64::new(0),
    reconnects: AtomicU64::new(0),
};

pub fn stats() -> &'static RateLimiterStats {
    &STATS
}

struct RedisLink {
    connection: Option<redis::aio::ConnectionManager>,
    next_attempt: Instant,
}

pub struct RedisRateLimiter {
    client: redis::Client,
    link: Mutex<RedisLink>,
    local: Arc<UserRateLimiter>,
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
        let local = create_keyed_limiter(requests_per_second, burst_size)
            .ok_or_else(|| redis::RedisError::from((redis::ErrorKind::ClientError, "invalid rate limit")))?;
        let connection = match client.get_connection_manager().await {
            Ok(connection) => Some(connection),
            Err(e) => {
                tracing::warn!(error = %e, scope, "Redis unavailable at startup; rate limiter will keep reconnecting");
                STATS.redis_errors.fetch_add(1, Ordering::Relaxed);
                None
            }
        };
        let emission_interval_ms = (1000 / requests_per_second.max(1)).max(1) as u64;
        let delay_tolerance_ms = emission_interval_ms * (burst_size.max(1) as u64 - 1);
        Ok(Self {
            client,
            link: Mutex::new(RedisLink { connection, next_attempt: Instant::now() + RECONNECT_EVERY }),
            local,
            script: redis::Script::new(GCRA_SCRIPT),
            scope: scope.to_string(),
            emission_interval_ms,
            delay_tolerance_ms,
        })
    }

    async fn connection(&self) -> Option<redis::aio::ConnectionManager> {
        {
            let mut link = self.link.lock().unwrap();
            if let Some(connection) = &link.connection {
                return Some(connection.clone());
            }
            let now = Instant::now();
            if now < link.next_attempt {
                return None;
            }
            link.next_attempt = now + RECONNECT_EVERY;
        }
        match self.client.get_connection_manager().await {
            Ok(connection) => {
                STATS.reconnects.fetch_add(1, Ordering::Relaxed);
                tracing::info!(scope = %self.scope, "Rate limiter reconnected to Redis");
                self.link.lock().unwrap().connection = Some(connection.clone());
                Some(connection)
            }
            Err(e) => {
                STATS.redis_errors.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(error = %e, scope = %self.scope, "Redis still unavailable; rate limiting locally");
                None
            }
        }
    }

    fn drop_connection(&self) {
        let mut link = self.link.lock().unwrap();
        link.connection = None;
        link.next_attempt = Instant::now() + RECONNECT_EVERY;
    }

    async fn check(&self, key: &Uuid) -> bool {
        let Some(mut connection) = self.connection().await else {
            STATS.local_fallbacks.fetch_add(1, Ordering::Relaxed);
            return self.local.check_key(key).is_ok();
        };
        STATS.redis_checks.fetch_add(1, Ordering::Relaxed);
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
                STATS.redis_errors.fetch_add(1, Ordering::Relaxed);
                STATS.local_fallbacks.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(error = %e, scope = %self.scope, "Redis rate limiter failed; rate limiting locally");
                self.drop_connection();
                self.local.check_key(key).is_ok()
            }
        }
    }
}

pub enum KeyedRateLimiter {
    InMemory(Arc<UserRateLimiter>),
    Redis(Box<RedisRateLimiter>),
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
        Some(url) => match RedisRateLimiter::connect(&url, scope, requests_per_second, burst_size).await {
            Ok(limiter) => {
                tracing::info!(scope, "Rate limiter backed by Redis");
                Some(KeyedRateLimiter::Redis(Box::new(limiter)))
            }
            Err(e) => {
                tracing::warn!(error = %e, scope, "REDIS_URL is invalid; falling back to in-memory rate limiter");
                create_keyed_limiter(requests_per_second, burst_size).map(KeyedRateLimiter::InMemory)
            }
        },
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
