use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

struct HeartbeatInner {
    origin: Instant,
    last_ms: AtomicU64,
    stale_after: Duration,
}

impl HeartbeatInner {
    fn is_fresh(&self) -> bool {
        let now = self.origin.elapsed().as_millis() as u64;
        let last = self.last_ms.load(Ordering::Acquire);
        now.saturating_sub(last) < self.stale_after.as_millis() as u64
    }
}

#[derive(Clone)]
pub struct Heartbeat {
    inner: Arc<HeartbeatInner>,
}

impl Heartbeat {
    pub fn new(stale_after: Duration) -> Self {
        Self {
            inner: Arc::new(HeartbeatInner {
                origin: Instant::now(),
                last_ms: AtomicU64::new(0),
                stale_after,
            }),
        }
    }

    pub fn beat(&self) {
        let elapsed = self.inner.origin.elapsed().as_millis() as u64;
        self.inner.last_ms.store(elapsed, Ordering::Release);
    }

    pub fn is_fresh(&self) -> bool {
        self.inner.is_fresh()
    }

    pub fn pulse_interval(&self) -> Duration {
        (self.inner.stale_after / 4).max(Duration::from_millis(10))
    }

    pub async fn keep_alive_for(&self, duration: Duration) {
        let step = self.pulse_interval();
        let deadline = Instant::now() + duration;
        loop {
            self.beat();
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            tokio::time::sleep((deadline - now).min(step)).await;
        }
    }
}

#[derive(Clone, Default)]
pub struct Liveness {
    watched: Arc<Mutex<Vec<Arc<HeartbeatInner>>>>,
}

impl Liveness {
    pub fn watch(self, heartbeat: Heartbeat) -> Self {
        self.lock().push(heartbeat.inner);
        self
    }

    pub fn register(&self, stale_after: Duration) -> Heartbeat {
        let heartbeat = Heartbeat::new(stale_after);
        self.lock().push(heartbeat.inner.clone());
        heartbeat
    }

    pub fn release(&self, heartbeat: &Heartbeat) {
        self.lock()
            .retain(|inner| !Arc::ptr_eq(inner, &heartbeat.inner));
    }

    pub fn is_alive(&self) -> bool {
        self.lock().iter().all(|inner| inner.is_fresh())
    }

    pub fn watched(&self) -> usize {
        self.lock().len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Arc<HeartbeatInner>>> {
        self.watched.lock().unwrap_or_else(|e| e.into_inner())
    }
}
