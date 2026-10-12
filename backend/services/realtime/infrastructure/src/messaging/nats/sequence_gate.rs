use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use tokio::sync::Notify;
use tokio::time::Instant;

const MAX_TRACKED_PARTITIONS: usize = 100_000;
const FIRST_TURN_SETTLE: Duration = Duration::from_millis(150);
const RESTORE_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_millis(100),
    Duration::from_millis(300),
    Duration::from_millis(700),
];

#[async_trait]
pub trait SequenceProgress: Send + Sync {
    async fn load(&self, partition: &str) -> Result<Option<i64>, String>;
    async fn save(&self, progress: &[(String, i64)]) -> Result<(), String>;
}

struct PartitionTurn {
    done: Option<i64>,
    waiting: BTreeSet<i64>,
    elected: Option<(i64, Instant)>,
    stalled_since: Option<Instant>,
    opened_at: Instant,
    restored: bool,
}

impl PartitionTurn {
    fn new() -> Self {
        Self {
            done: None,
            waiting: BTreeSet::new(),
            elected: None,
            stalled_since: None,
            opened_at: Instant::now(),
            restored: false,
        }
    }

    fn active(&self) -> bool {
        !self.waiting.is_empty() || self.elected.is_some()
    }
}

enum Turn {
    Go,
    Late,
    Wait(Instant),
}

pub struct SequenceGate {
    last: Mutex<HashMap<String, PartitionTurn>>,
    advanced: Notify,
    max_wait: Duration,
    settle: Duration,
    progress: Option<Arc<dyn SequenceProgress>>,
    unsaved: Mutex<HashMap<String, i64>>,
}

fn turn_for<'a>(last: &'a mut HashMap<String, PartitionTurn>, partition: &str) -> &'a mut PartitionTurn {
    if last.len() >= MAX_TRACKED_PARTITIONS && !last.contains_key(partition) {
        last.retain(|_, turn| turn.active());
    }
    last.entry(partition.to_string()).or_insert_with(PartitionTurn::new)
}

impl SequenceGate {
    pub fn new(max_wait: Duration) -> Self {
        Self::with_settle(max_wait, FIRST_TURN_SETTLE)
    }

    pub fn with_settle(max_wait: Duration, settle: Duration) -> Self {
        Self {
            last: Mutex::new(HashMap::new()),
            advanced: Notify::new(),
            max_wait,
            settle,
            progress: None,
            unsaved: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_progress(max_wait: Duration, progress: Arc<dyn SequenceProgress>) -> Self {
        Self { progress: Some(progress), ..Self::new(max_wait) }
    }

    async fn restore(&self, partition: &str) {
        let Some(progress) = &self.progress else {
            return;
        };
        {
            let last = self.last.lock().unwrap_or_else(|e| e.into_inner());
            if last.get(partition).is_some_and(|turn| turn.restored || turn.done.is_some()) {
                return;
            }
        }
        let mut stored = progress.load(partition).await;
        for delay in RESTORE_RETRY_DELAYS {
            if stored.is_ok() {
                break;
            }
            tokio::time::sleep(delay).await;
            stored = progress.load(partition).await;
        }
        match stored {
            Ok(stored) => {
                let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
                let turn = turn_for(&mut last, partition);
                turn.restored = true;
                if turn.done.is_none() {
                    turn.done = Some(stored.unwrap_or(0));
                    turn.elected = None;
                }
            }
            Err(e) => {
                tracing::warn!(partition, error = %e, "Failed to restore partition progress; this event falls back to the settle window");
            }
        }
    }

    pub async fn save_progress(&self) {
        let Some(progress) = &self.progress else {
            return;
        };
        let pending: Vec<(String, i64)> = {
            let mut unsaved = self.unsaved.lock().unwrap_or_else(|e| e.into_inner());
            unsaved.drain().collect()
        };
        if pending.is_empty() {
            return;
        }
        if let Err(e) = progress.save(&pending).await {
            tracing::warn!(error = %e, partitions = pending.len(), "Failed to save partition progress; will retry");
            let mut unsaved = self.unsaved.lock().unwrap_or_else(|e| e.into_inner());
            for (partition, seq) in pending {
                let entry = unsaved.entry(partition).or_insert(seq);
                *entry = (*entry).max(seq);
            }
        }
    }

    fn turn(&self, partition: &str, seq: i64) -> Turn {
        let now = Instant::now();
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        let turn = turn_for(&mut last, partition);
        if let Some(done) = turn.done {
            if seq <= done {
                turn.waiting.remove(&seq);
                return Turn::Late;
            }
            if seq == done + 1 {
                turn.waiting.remove(&seq);
                return Turn::Go;
            }
        }
        turn.waiting.insert(seq);
        if let Some((elected, at)) = turn.elected {
            if elected == seq {
                turn.waiting.remove(&seq);
                return Turn::Go;
            }
            if now < at + self.max_wait {
                return Turn::Wait(at + self.max_wait);
            }
            turn.elected = None;
        }
        let elect_at = match turn.done {
            None => turn.opened_at + self.settle,
            Some(_) => *turn.stalled_since.get_or_insert(now) + self.max_wait,
        };
        if now < elect_at {
            return Turn::Wait(elect_at);
        }
        let lowest = *turn.waiting.first().unwrap_or(&seq);
        turn.elected = Some((lowest, now));
        if lowest == seq {
            turn.waiting.remove(&seq);
            return Turn::Go;
        }
        self.advanced.notify_waiters();
        Turn::Wait(now + self.max_wait)
    }

    pub async fn wait_turn(&self, partition: &str, seq: i64) -> bool {
        self.restore(partition).await;
        loop {
            let advanced = self.advanced.notified();
            tokio::pin!(advanced);
            advanced.as_mut().enable();
            let wake = match self.turn(partition, seq) {
                Turn::Go => return true,
                Turn::Late => return false,
                Turn::Wait(wake) => wake,
            };
            let _ = tokio::time::timeout_at(wake, advanced).await;
        }
    }

    pub fn done(&self, partition: &str, seq: i64) {
        {
            let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
            let turn = turn_for(&mut last, partition);
            let done = turn.done.map_or(seq, |done| done.max(seq));
            turn.done = Some(done);
            if turn.elected.is_some_and(|(elected, _)| elected <= done) {
                turn.elected = None;
            }
            turn.waiting.retain(|waiting| *waiting > done);
            turn.stalled_since = None;
        }
        if self.progress.is_some() {
            let mut unsaved = self.unsaved.lock().unwrap_or_else(|e| e.into_inner());
            let entry = unsaved.entry(partition.to_string()).or_insert(seq);
            *entry = (*entry).max(seq);
        }
        self.advanced.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn lets_the_first_event_of_a_partition_through_after_the_settle_window() {
        let gate = SequenceGate::with_settle(Duration::from_secs(5), Duration::from_millis(20));
        tokio::time::timeout(Duration::from_millis(200), gate.wait_turn("p", 7)).await.unwrap();
    }

    #[derive(Default)]
    struct MemoryProgress {
        stored: Mutex<HashMap<String, i64>>,
        fail: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl SequenceProgress for MemoryProgress {
        async fn load(&self, partition: &str) -> Result<Option<i64>, String> {
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("down".into());
            }
            Ok(self.stored.lock().unwrap().get(partition).copied())
        }

        async fn save(&self, progress: &[(String, i64)]) -> Result<(), String> {
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("down".into());
            }
            let mut stored = self.stored.lock().unwrap();
            for (partition, seq) in progress {
                let entry = stored.entry(partition.clone()).or_insert(*seq);
                *entry = (*entry).max(*seq);
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn lets_only_the_lowest_waiting_event_through_after_a_gap_times_out() {
        let gate = Arc::new(SequenceGate::with_settle(Duration::from_millis(50), Duration::ZERO));
        gate.done("p", 9);
        let higher = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 12).await }
        });
        let lower = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 11).await }
        });
        assert!(tokio::time::timeout(Duration::from_millis(500), lower).await.unwrap().unwrap());
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!higher.is_finished());
        gate.done("p", 11);
        assert!(tokio::time::timeout(Duration::from_millis(500), higher).await.unwrap().unwrap());
    }

    #[tokio::test]
    async fn keeps_active_partitions_when_the_tracking_cap_is_reached() {
        let gate = Arc::new(SequenceGate::with_settle(Duration::from_secs(5), Duration::ZERO));
        gate.done("busy", 1);
        let waiting = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("busy", 3).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        for n in 0..MAX_TRACKED_PARTITIONS {
            gate.done(&format!("idle-{n}"), 1);
        }
        assert!(!waiting.is_finished());
        gate.done("busy", 2);
        assert!(tokio::time::timeout(Duration::from_millis(500), waiting).await.unwrap().unwrap());
        assert!(gate.last.lock().unwrap().len() < MAX_TRACKED_PARTITIONS);
    }

    #[tokio::test]
    async fn reports_an_event_that_arrives_after_a_later_one_was_sent() {
        let gate = SequenceGate::with_settle(Duration::from_millis(50), Duration::ZERO);
        gate.done("p", 9);
        assert!(gate.wait_turn("p", 11).await);
        gate.done("p", 11);
        assert!(!gate.wait_turn("p", 10).await);
        assert!(!gate.wait_turn("p", 11).await);
        assert!(gate.wait_turn("p", 12).await);
    }

    #[tokio::test]
    async fn continues_a_partition_from_its_saved_progress() {
        let progress = Arc::new(MemoryProgress::default());
        progress.stored.lock().unwrap().insert("p".into(), 10);
        let gate = Arc::new(SequenceGate::with_progress(Duration::from_secs(5), progress));
        let later = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 12).await }
        });
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 11)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!later.is_finished());
        gate.done("p", 11);
        tokio::time::timeout(Duration::from_millis(500), later).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn starts_an_unsaved_partition_from_the_first_sequence() {
        let progress = Arc::new(MemoryProgress::default());
        let gate = Arc::new(SequenceGate::with_progress(Duration::from_secs(5), progress));
        let second = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 2).await }
        });
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!second.is_finished());
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 1)).await.unwrap();
        gate.done("p", 1);
        tokio::time::timeout(Duration::from_millis(500), second).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn saves_the_highest_finished_sequence_and_retries_after_a_failure() {
        let progress = Arc::new(MemoryProgress::default());
        let gate = SequenceGate::with_progress(Duration::from_secs(5), progress.clone());
        gate.done("p", 3);
        gate.done("p", 2);
        progress.fail.store(true, std::sync::atomic::Ordering::SeqCst);
        gate.save_progress().await;
        assert!(progress.stored.lock().unwrap().is_empty());
        progress.fail.store(false, std::sync::atomic::Ordering::SeqCst);
        gate.done("q", 7);
        gate.save_progress().await;
        let stored = progress.stored.lock().unwrap().clone();
        assert_eq!(stored.get("p"), Some(&3));
        assert_eq!(stored.get("q"), Some(&7));
    }

    #[tokio::test]
    async fn keeps_trying_to_restore_progress_after_a_failed_read() {
        let progress = Arc::new(MemoryProgress::default());
        progress.stored.lock().unwrap().insert("p".into(), 10);
        progress.fail.store(true, std::sync::atomic::Ordering::SeqCst);
        let gate = Arc::new(SequenceGate::with_progress(Duration::from_secs(5), progress.clone()));
        let recovering = tokio::spawn({
            let progress = progress.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(150)).await;
                progress.fail.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        });
        let later = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 12).await }
        });
        recovering.await.unwrap();
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(!later.is_finished());
        assert!(tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 11)).await.unwrap());
    }

    #[tokio::test]
    async fn falls_back_to_the_settle_window_when_progress_cannot_be_read() {
        let progress = Arc::new(MemoryProgress::default());
        progress.fail.store(true, std::sync::atomic::Ordering::SeqCst);
        let gate = SequenceGate::with_progress(Duration::from_secs(5), progress);
        tokio::time::timeout(Duration::from_secs(3), gate.wait_turn("p", 40)).await.unwrap();
    }

    #[tokio::test]
    async fn starts_a_new_partition_with_the_lowest_sequence_that_arrives_together() {
        let gate = Arc::new(SequenceGate::with_settle(Duration::from_secs(5), Duration::from_millis(50)));
        let higher = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 11).await }
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        let lower = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 10).await }
        });
        tokio::time::timeout(Duration::from_millis(500), lower).await.unwrap().unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!higher.is_finished());
        gate.done("p", 10);
        tokio::time::timeout(Duration::from_millis(500), higher).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn serializes_the_first_events_of_a_new_partition() {
        let gate = Arc::new(SequenceGate::with_settle(Duration::from_secs(5), Duration::ZERO));
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 10)).await.unwrap();
        let second = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 11).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!second.is_finished());
        gate.done("p", 10);
        tokio::time::timeout(Duration::from_millis(500), second).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn lets_a_redelivered_first_event_through_again() {
        let gate = SequenceGate::with_settle(Duration::from_secs(5), Duration::ZERO);
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 4)).await.unwrap();
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 4)).await.unwrap();
    }

    #[tokio::test]
    async fn stops_waiting_for_a_first_event_that_never_finishes() {
        let gate = SequenceGate::with_settle(Duration::from_millis(50), Duration::ZERO);
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 4)).await.unwrap();
        tokio::time::timeout(Duration::from_millis(500), gate.wait_turn("p", 5)).await.unwrap();
    }

    #[tokio::test]
    async fn holds_a_later_event_until_the_previous_one_is_done() {
        let gate = Arc::new(SequenceGate::new(Duration::from_secs(5)));
        gate.done("p", 1);
        let waiting = tokio::spawn({
            let gate = gate.clone();
            async move { gate.wait_turn("p", 3).await }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!waiting.is_finished());
        gate.done("p", 2);
        tokio::time::timeout(Duration::from_millis(500), waiting).await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn gives_up_waiting_for_a_missing_event() {
        let gate = SequenceGate::new(Duration::from_millis(50));
        gate.done("p", 1);
        tokio::time::timeout(Duration::from_millis(500), gate.wait_turn("p", 5)).await.unwrap();
    }

    #[tokio::test]
    async fn other_partitions_never_wait() {
        let gate = SequenceGate::with_settle(Duration::from_secs(5), Duration::ZERO);
        gate.done("p", 1);
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("q", 9)).await.unwrap();
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 2)).await.unwrap();
    }
}
