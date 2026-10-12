use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::Instant;

const MAX_TRACKED_PARTITIONS: usize = 100_000;
const FIRST_TURN_SETTLE: Duration = Duration::from_millis(150);

struct PartitionTurn {
    done: Option<i64>,
    first: Option<i64>,
    contenders: BTreeSet<i64>,
    opened_at: Instant,
}

impl PartitionTurn {
    fn new() -> Self {
        Self { done: None, first: None, contenders: BTreeSet::new(), opened_at: Instant::now() }
    }
}

enum Turn {
    Go,
    Wait(Option<Instant>),
}

pub struct SequenceGate {
    last: Mutex<HashMap<String, PartitionTurn>>,
    advanced: Notify,
    max_wait: Duration,
    settle: Duration,
}

impl SequenceGate {
    pub fn new(max_wait: Duration) -> Self {
        Self::with_settle(max_wait, FIRST_TURN_SETTLE)
    }

    pub fn with_settle(max_wait: Duration, settle: Duration) -> Self {
        Self { last: Mutex::new(HashMap::new()), advanced: Notify::new(), max_wait, settle }
    }

    fn turn(&self, partition: &str, seq: i64) -> Turn {
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if last.len() >= MAX_TRACKED_PARTITIONS && !last.contains_key(partition) {
            last.clear();
        }
        let turn = last.entry(partition.to_string()).or_insert_with(PartitionTurn::new);
        if let Some(done) = turn.done {
            return if seq <= done + 1 { Turn::Go } else { Turn::Wait(None) };
        }
        if let Some(first) = turn.first {
            return if first == seq { Turn::Go } else { Turn::Wait(None) };
        }
        turn.contenders.insert(seq);
        let settled_at = turn.opened_at + self.settle;
        if Instant::now() < settled_at {
            return Turn::Wait(Some(settled_at));
        }
        if turn.contenders.first() == Some(&seq) {
            turn.first = Some(seq);
            turn.contenders.clear();
            return Turn::Go;
        }
        Turn::Wait(None)
    }

    fn forget_contender(&self, partition: &str, seq: i64) {
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(turn) = last.get_mut(partition) {
            turn.contenders.remove(&seq);
        }
    }

    pub async fn wait_turn(&self, partition: &str, seq: i64) {
        let deadline = Instant::now() + self.max_wait;
        loop {
            let advanced = self.advanced.notified();
            tokio::pin!(advanced);
            advanced.as_mut().enable();
            let wake = match self.turn(partition, seq) {
                Turn::Go => return,
                Turn::Wait(settled_at) => settled_at.map_or(deadline, |at| at.min(deadline)),
            };
            if tokio::time::timeout_at(wake, advanced).await.is_err() && wake >= deadline {
                self.forget_contender(partition, seq);
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
            let turn = last.entry(partition.to_string()).or_insert_with(PartitionTurn::new);
            turn.done = Some(turn.done.map_or(seq, |done| done.max(seq)));
            turn.first = None;
            turn.contenders.clear();
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
