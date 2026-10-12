use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use infrastructure_common::database::postgres::repositories::outbox_record::save_outbox_records;
use infrastructure_common::outbox::dispatcher::OutboxDispatcher;
use ports::outbox::record::{AggregateType, OutboxRecord, PartitionSequence};
use ports::outbox_publisher::{OutboxPublisher, OutboxPublisherError};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, Executor, PgConnection, Pool, Postgres};
use uuid::Uuid;

const ADMIN_URL: &str = "RIFT_TEST_DATABASE_URL";

struct TestDb {
    admin: PgConnectOptions,
    name: String,
    pool: Pool<Postgres>,
}

impl TestDb {
    async fn new() -> Self {
        let url = std::env::var(ADMIN_URL).expect("RIFT_TEST_DATABASE_URL must point at a Postgres admin database");
        let admin = PgConnectOptions::from_str(&url).unwrap();
        let name = format!("outbox_test_{}", Uuid::new_v4().simple());
        let mut conn = PgConnection::connect_with(&admin).await.unwrap();
        conn.execute(format!("CREATE DATABASE {name}").as_str()).await.unwrap();
        conn.close().await.unwrap();
        let options = admin.clone().database(&name).options([("search_path", "messaging")]);
        let pool = PgPoolOptions::new().max_connections(16).connect_with(options).await.unwrap();
        pool.execute("CREATE SCHEMA messaging").await.unwrap();
        let migrations = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../services/messaging/migrations");
        Migrator::new(migrations).await.unwrap().run(&pool).await.unwrap();
        Self { admin, name, pool }
    }

    async fn drop(self) {
        self.pool.close().await;
        let mut conn = PgConnection::connect_with(&self.admin).await.unwrap();
        conn.execute(format!("DROP DATABASE {} WITH (FORCE)", self.name).as_str()).await.unwrap();
        conn.close().await.unwrap();
    }
}

fn channel_event(channel: Uuid, label: &str) -> OutboxRecord {
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: AggregateType::Message,
        parent_id: Some(channel),
        parent_type: Some(AggregateType::Channel),
        event_type: label.to_string(),
        payload: serde_json::json!({}),
        occurred_at: Utc::now(),
        request_id: None,
    }
}

async fn save(pool: &Pool<Postgres>, records: &[OutboxRecord]) {
    let mut tx = pool.begin().await.unwrap();
    save_outbox_records(&mut tx, records).await.unwrap();
    tx.commit().await.unwrap();
}

#[derive(Default)]
struct FakePublisher {
    failing: Mutex<HashSet<String>>,
    attempts: Mutex<HashMap<String, u32>>,
    published: Mutex<Vec<(String, Option<i64>)>>,
}

impl FakePublisher {
    fn fail(&self, label: &str) {
        self.failing.lock().unwrap().insert(label.to_string());
    }

    fn recover(&self, label: &str) {
        self.failing.lock().unwrap().remove(label);
    }

    fn published(&self) -> Vec<String> {
        self.published.lock().unwrap().iter().map(|(label, _)| label.clone()).collect()
    }

    fn attempts(&self, label: &str) -> u32 {
        self.attempts.lock().unwrap().get(label).copied().unwrap_or(0)
    }
}

#[async_trait]
impl OutboxPublisher for FakePublisher {
    async fn publish(&self, record: &OutboxRecord, sequence: Option<PartitionSequence>) -> Result<(), OutboxPublisherError> {
        *self.attempts.lock().unwrap().entry(record.event_type.clone()).or_default() += 1;
        if self.failing.lock().unwrap().contains(&record.event_type) {
            return Err(OutboxPublisherError::Connection("broker down".into()));
        }
        self.published.lock().unwrap().push((record.event_type.clone(), sequence.map(|s| s.seq)));
        Ok(())
    }
}

fn dispatcher(pool: &Pool<Postgres>, publisher: &Arc<FakePublisher>, max_attempts: i16) -> OutboxDispatcher<FakePublisher> {
    OutboxDispatcher::new("messaging".into(), pool.clone(), publisher.clone(), max_attempts, 0, 1)
}

async fn drain(dispatcher: &OutboxDispatcher<FakePublisher>, rounds: usize) {
    for _ in 0..rounds {
        if !dispatcher.process_one().await.unwrap() {
            return;
        }
    }
}

#[tokio::test]
#[ignore = "needs Postgres through RIFT_TEST_DATABASE_URL"]
async fn concurrent_writers_get_gapless_sequences_in_commit_order() {
    let db = TestDb::new().await;
    let channel = Uuid::new_v4();
    let writers = (0..8).map(|writer| {
        let pool = db.pool.clone();
        tokio::spawn(async move {
            for n in 0..25 {
                save(&pool, &[channel_event(channel, &format!("w{writer}-{n}"))]).await;
            }
        })
    });
    for writer in futures::future::join_all(writers).await {
        writer.unwrap();
    }

    let rows: Vec<(i64, Option<i64>)> =
        sqlx::query_as("SELECT seq, partition_seq FROM outbox WHERE partition_key = $1 ORDER BY seq")
            .bind(channel)
            .fetch_all(&db.pool)
            .await
            .unwrap();
    let sequences: Vec<i64> = rows.iter().map(|(_, seq)| seq.unwrap()).collect();
    assert_eq!(sequences, (1..=200).collect::<Vec<i64>>());
    db.drop().await;
}

#[tokio::test]
#[ignore = "needs Postgres through RIFT_TEST_DATABASE_URL"]
async fn a_failing_event_holds_back_only_its_own_partition() {
    let db = TestDb::new().await;
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    save(&db.pool, &[channel_event(a, "a1")]).await;
    save(&db.pool, &[channel_event(b, "b1")]).await;
    save(&db.pool, &[channel_event(a, "a2")]).await;
    save(&db.pool, &[channel_event(b, "b2")]).await;

    let publisher = Arc::new(FakePublisher::default());
    publisher.fail("a1");
    let dispatcher = dispatcher(&db.pool, &publisher, 5);
    drain(&dispatcher, 20).await;
    assert_eq!(publisher.published(), vec!["b1", "b2"]);
    assert_eq!(publisher.attempts("a2"), 0);

    publisher.recover("a1");
    drain(&dispatcher, 20).await;
    assert_eq!(publisher.published(), vec!["b1", "b2", "a1", "a2"]);
    let sequences: Vec<Option<i64>> = publisher.published.lock().unwrap().iter().map(|(_, seq)| *seq).collect();
    assert_eq!(sequences, vec![Some(1), Some(2), Some(1), Some(2)]);
    db.drop().await;
}

#[tokio::test]
#[ignore = "needs Postgres through RIFT_TEST_DATABASE_URL"]
async fn an_event_is_kept_after_running_out_of_attempts() {
    let db = TestDb::new().await;
    let channel = Uuid::new_v4();
    save(&db.pool, &[channel_event(channel, "stuck")]).await;
    save(&db.pool, &[channel_event(channel, "next")]).await;

    let publisher = Arc::new(FakePublisher::default());
    publisher.fail("stuck");
    let dispatcher = dispatcher(&db.pool, &publisher, 2);
    drain(&dispatcher, 6).await;
    assert!(publisher.attempts("stuck") >= 3);
    assert!(publisher.published().is_empty());
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox WHERE processed_at IS NULL")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(pending, 2);

    publisher.recover("stuck");
    drain(&dispatcher, 10).await;
    assert_eq!(publisher.published(), vec!["stuck", "next"]);
    db.drop().await;
}

#[tokio::test]
#[ignore = "needs Postgres through RIFT_TEST_DATABASE_URL"]
async fn parallel_dispatchers_publish_each_event_once_in_partition_order() {
    let db = TestDb::new().await;
    let channels: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
    for n in 0..20 {
        for (c, channel) in channels.iter().enumerate() {
            save(&db.pool, &[channel_event(*channel, &format!("c{c}-{n:02}"))]).await;
        }
    }

    let publisher = Arc::new(FakePublisher::default());
    let workers = (0..4).map(|_| {
        let dispatcher = dispatcher(&db.pool, &publisher, 5);
        let publisher = publisher.clone();
        tokio::spawn(async move {
            for _ in 0..2000 {
                if publisher.published.lock().unwrap().len() >= 100 {
                    return;
                }
                if !dispatcher.process_one().await.unwrap() {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            }
        })
    });
    for worker in futures::future::join_all(workers).await {
        worker.unwrap();
    }

    let published = publisher.published();
    assert_eq!(published.len(), 100);
    assert_eq!(published.iter().collect::<HashSet<_>>().len(), 100);
    for c in 0..channels.len() {
        let prefix = format!("c{c}-");
        let order: Vec<&String> = published.iter().filter(|label| label.starts_with(&prefix)).collect();
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(order, sorted);
    }
    db.drop().await;
}
