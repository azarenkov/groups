use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::Instant;

const MAX_TRACKED_PARTITIONS: usize = 100_000;

pub struct SequenceGate {
    last: Mutex<HashMap<String, i64>>,
    advanced: Notify,
    max_wait: Duration,
}

impl SequenceGate {
    pub fn new(max_wait: Duration) -> Self {
        Self { last: Mutex::new(HashMap::new()), advanced: Notify::new(), max_wait }
    }

    fn ready(&self, partition: &str, seq: i64) -> bool {
        let last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        match last.get(partition) {
            None => true,
            Some(&done) => seq <= done + 1,
        }
    }

    pub async fn wait_turn(&self, partition: &str, seq: i64) {
        let deadline = Instant::now() + self.max_wait;
        loop {
            let advanced = self.advanced.notified();
            tokio::pin!(advanced);
            advanced.as_mut().enable();
            if self.ready(partition, seq) {
                return;
            }
            if tokio::time::timeout_at(deadline, advanced).await.is_err() {
                return;
            }
        }
    }

    pub fn done(&self, partition: &str, seq: i64) {
        {
            let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
            if last.len() >= MAX_TRACKED_PARTITIONS && !last.contains_key(partition) {
                last.clear();
            }
            let entry = last.entry(partition.to_string()).or_insert(seq);
            if seq > *entry {
                *entry = seq;
            }
        }
        self.advanced.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn lets_the_first_event_of_a_partition_through() {
        let gate = SequenceGate::new(Duration::from_secs(5));
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 7)).await.unwrap();
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
        let gate = SequenceGate::new(Duration::from_secs(5));
        gate.done("p", 1);
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("q", 9)).await.unwrap();
        tokio::time::timeout(Duration::from_millis(100), gate.wait_turn("p", 2)).await.unwrap();
    }
}
