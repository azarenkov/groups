use std::{
    future::Future,
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use actix_web::{App, HttpResponse, HttpServer, dev::Server, web};
use futures::future::join_all;
use tokio::sync::Mutex;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type Probe = Arc<dyn Fn() -> BoxFuture<'static, bool> + Send + Sync>;

const DEFAULT_PROBE_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_PROBE_CACHE_MS: u64 = 5_000;

fn env_duration_ms(name: &str, default_ms: u64) -> Duration {
    let ms = std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(default_ms);
    Duration::from_millis(ms)
}

struct CachedProbe {
    probe: Probe,
    timeout: Duration,
    ttl: Duration,
    last: Mutex<Option<(Instant, bool)>>,
}

impl CachedProbe {
    async fn check(&self) -> bool {
        let requested_at = Instant::now();
        let mut last = self.last.lock().await;
        if let Some((at, ok)) = *last {
            if at >= requested_at || at.elapsed() < self.ttl {
                return ok;
            }
        }
        let ok = matches!(
            tokio::time::timeout(self.timeout, (self.probe)()).await,
            Ok(true)
        );
        *last = Some((Instant::now(), ok));
        ok
    }
}

#[derive(Clone)]
pub struct Readiness {
    started: Arc<AtomicBool>,
    probes: Arc<Vec<CachedProbe>>,
}

impl Readiness {
    pub fn builder() -> ReadinessBuilder {
        ReadinessBuilder {
            probes: Vec::new(),
            probe_timeout: env_duration_ms("HEALTH_PROBE_TIMEOUT_MS", DEFAULT_PROBE_TIMEOUT_MS),
            cache_ttl: env_duration_ms("HEALTH_PROBE_CACHE_MS", DEFAULT_PROBE_CACHE_MS),
        }
    }

    pub fn mark_ready(&self) {
        self.started.store(true, Ordering::Release);
    }

    pub async fn check(&self) -> bool {
        if !self.started.load(Ordering::Acquire) {
            return false;
        }
        join_all(self.probes.iter().map(CachedProbe::check))
            .await
            .into_iter()
            .all(|ok| ok)
    }
}

pub struct ReadinessBuilder {
    probes: Vec<Probe>,
    probe_timeout: Duration,
    cache_ttl: Duration,
}

impl ReadinessBuilder {
    pub fn add_probe<F, Fut>(mut self, probe: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        let probe = Arc::new(probe);
        self.probes.push(Arc::new(move || {
            let probe = probe.clone();
            Box::pin(async move { probe().await }) as BoxFuture<'static, bool>
        }));
        self
    }

    pub fn probe_timeout(mut self, timeout: Duration) -> Self {
        self.probe_timeout = timeout;
        self
    }

    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    pub fn build(self) -> Readiness {
        let probes = self
            .probes
            .into_iter()
            .map(|probe| CachedProbe {
                probe,
                timeout: self.probe_timeout,
                ttl: self.cache_ttl,
                last: Mutex::new(None),
            })
            .collect();
        Readiness {
            started: Arc::new(AtomicBool::new(false)),
            probes: Arc::new(probes),
        }
    }
}

pub use infrastructure_common::health::{Heartbeat, Liveness};

pub fn bind(port: u16, readiness: Readiness, liveness: Liveness) -> io::Result<Server> {
    let readiness_data = web::Data::new(readiness);
    let liveness_data = web::Data::new(liveness);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(readiness_data.clone())
            .app_data(liveness_data.clone())
            .service(web::resource("/livez").to(livez))
            .service(web::resource("/readyz").to(readyz))
    })
    .workers(1)
    .bind(("0.0.0.0", port))?
    .run();
    tracing::info!(port, "Health server listening");
    Ok(server)
}

pub async fn livez(liveness: Option<web::Data<Liveness>>) -> HttpResponse {
    match liveness {
        Some(liveness) if !liveness.is_alive() => HttpResponse::ServiceUnavailable().finish(),
        _ => HttpResponse::Ok().finish(),
    }
}

pub async fn readyz(readiness: web::Data<Readiness>) -> HttpResponse {
    if readiness.check().await {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::ServiceUnavailable().finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn counting_probe(calls: Arc<AtomicUsize>, result: bool) -> impl Fn() -> BoxFuture<'static, bool> + Send + Sync {
        move || {
            calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { result }) as BoxFuture<'static, bool>
        }
    }

    #[tokio::test]
    async fn not_ready_until_marked() {
        let readiness = Readiness::builder().build();
        assert!(!readiness.check().await);
        readiness.mark_ready();
        assert!(readiness.check().await);
    }

    #[tokio::test]
    async fn slow_probe_times_out_as_not_ready() {
        let readiness = Readiness::builder()
            .probe_timeout(Duration::from_millis(20))
            .cache_ttl(Duration::ZERO)
            .add_probe(|| async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                true
            })
            .build();
        readiness.mark_ready();
        let started = Instant::now();
        assert!(!readiness.check().await);
        assert!(started.elapsed() < Duration::from_millis(150));
    }

    #[tokio::test]
    async fn probe_result_is_cached_for_ttl() {
        let calls = Arc::new(AtomicUsize::new(0));
        let readiness = Readiness::builder()
            .cache_ttl(Duration::from_millis(100))
            .add_probe(counting_probe(calls.clone(), true))
            .build();
        readiness.mark_ready();
        assert!(readiness.check().await);
        assert!(readiness.check().await);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(readiness.check().await);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn failed_probe_is_cached_too() {
        let calls = Arc::new(AtomicUsize::new(0));
        let readiness = Readiness::builder()
            .cache_ttl(Duration::from_secs(5))
            .add_probe(counting_probe(calls.clone(), false))
            .build();
        readiness.mark_ready();
        assert!(!readiness.check().await);
        assert!(!readiness.check().await);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn zero_ttl_reevaluates_sequential_checks() {
        let calls = Arc::new(AtomicUsize::new(0));
        let readiness = Readiness::builder()
            .cache_ttl(Duration::ZERO)
            .add_probe(counting_probe(calls.clone(), true))
            .build();
        readiness.mark_ready();
        assert!(readiness.check().await);
        assert!(readiness.check().await);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn concurrent_checks_share_one_evaluation() {
        let calls = Arc::new(AtomicUsize::new(0));
        let readiness = Readiness::builder()
            .cache_ttl(Duration::ZERO)
            .add_probe({
                let calls = calls.clone();
                move || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Box::pin(async {
                        tokio::time::sleep(Duration::from_millis(30)).await;
                        true
                    }) as BoxFuture<'static, bool>
                }
            })
            .build();
        readiness.mark_ready();
        let results = join_all((0..5).map(|_| readiness.check())).await;
        assert!(results.into_iter().all(|ok| ok));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn heartbeat_goes_stale_without_beats() {
        let heartbeat = Heartbeat::new(Duration::from_millis(40));
        let liveness = Liveness::default().watch(heartbeat.clone());
        assert!(liveness.is_alive());
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(!liveness.is_alive());
        heartbeat.beat();
        assert!(liveness.is_alive());
    }

    #[tokio::test]
    async fn keep_alive_for_pulses_through_long_waits() {
        let heartbeat = Heartbeat::new(Duration::from_millis(40));
        let liveness = Liveness::default().watch(heartbeat.clone());
        let waiter = tokio::spawn({
            let heartbeat = heartbeat.clone();
            async move { heartbeat.keep_alive_for(Duration::from_millis(150)).await }
        });
        for _ in 0..5 {
            tokio::time::sleep(Duration::from_millis(25)).await;
            assert!(liveness.is_alive());
        }
        waiter.await.unwrap();
        assert!(liveness.is_alive());
    }

    #[tokio::test]
    async fn liveness_requires_every_heartbeat_to_be_fresh() {
        let fresh = Heartbeat::new(Duration::from_secs(5));
        let stale = Heartbeat::new(Duration::from_millis(10));
        let liveness = Liveness::default().watch(fresh).watch(stale);
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!liveness.is_alive());
        assert!(Liveness::default().is_alive());
    }

    #[tokio::test]
    async fn dropped_registered_heartbeat_stays_watched_until_released() {
        let liveness = Liveness::default();
        let worker = liveness.register(Duration::from_millis(10));
        assert_eq!(liveness.watched(), 1);
        let abandoned = worker.clone();
        drop(worker);
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!liveness.is_alive());
        assert_eq!(liveness.watched(), 1);
        liveness.release(&abandoned);
        assert!(liveness.is_alive());
        assert_eq!(liveness.watched(), 0);
    }

    #[tokio::test]
    async fn probes_run_concurrently() {
        let readiness = Readiness::builder()
            .cache_ttl(Duration::ZERO)
            .add_probe(|| async {
                tokio::time::sleep(Duration::from_millis(60)).await;
                true
            })
            .add_probe(|| async {
                tokio::time::sleep(Duration::from_millis(60)).await;
                true
            })
            .build();
        readiness.mark_ready();
        let started = Instant::now();
        assert!(readiness.check().await);
        assert!(started.elapsed() < Duration::from_millis(110));
    }
}
