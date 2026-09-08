use std::{
    sync::{
        Arc, Mutex, Weak,
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
}

enum Watched {
    Held(Arc<HeartbeatInner>),
    Registered(Weak<HeartbeatInner>),
}

impl Watched {
    fn live(&self) -> Option<Arc<HeartbeatInner>> {
        match self {
            Self::Held(inner) => Some(inner.clone()),
            Self::Registered(weak) => weak.upgrade(),
        }
    }
}

#[derive(Clone, Default)]
pub struct Liveness {
    watched: Arc<Mutex<Vec<Watched>>>,
}

impl Liveness {
    pub fn watch(self, heartbeat: Heartbeat) -> Self {
        self.lock().push(Watched::Held(heartbeat.inner));
        self
    }

    pub fn register(&self, stale_after: Duration) -> Heartbeat {
        let heartbeat = Heartbeat::new(stale_after);
        self.lock()
            .push(Watched::Registered(Arc::downgrade(&heartbeat.inner)));
        heartbeat
    }

    pub fn is_alive(&self) -> bool {
        let mut watched = self.lock();
        watched.retain(|entry| entry.live().is_some());
        watched
            .iter()
            .all(|entry| entry.live().is_none_or(|inner| inner.is_fresh()))
    }

    pub fn watched(&self) -> usize {
        let mut watched = self.lock();
        watched.retain(|entry| entry.live().is_some());
        watched.len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Watched>> {
        self.watched.lock().unwrap_or_else(|e| e.into_inner())
    }
}
