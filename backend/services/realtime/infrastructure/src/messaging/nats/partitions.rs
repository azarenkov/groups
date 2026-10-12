use realtime_core::ports::blocks::RelationDirectory;
use std::collections::HashMap;
use infrastructure_common::health::Liveness;
use std::time::Duration;

use async_nats::jetstream::{Context, kv};
use futures::StreamExt;
use tokio::sync::watch;

use ports::event_consumer::StartPosition;

use std::sync::Arc;

use realtime_core::{ports::directory::RecipientDirectory, ports::projection::RealtimeProjection, ports::publisher::RealtimePublisher, usecases::dispatch_event::DispatchEvent};

use crate::messaging::nats::consumer::{ConsumerSpec, run_consumer};
use crate::messaging::nats::sequence_gate::SequenceGate;

#[derive(Debug, Clone)]
pub struct DispatcherSettings {
    pub user_stream: String,
    pub channel_stream: String,
    pub message_stream: String,
    pub dm_message_stream: String,
    pub community_stream: String,
    pub consumer_prefix: String,
    pub max_deliver: u32,
}

const BUCKET: &str = "dispatcher-partitions";
const SEQUENCE_WAIT: Duration = Duration::from_millis(1500);
const LEASE_TTL: Duration = Duration::from_secs(20);
const TICK: Duration = Duration::from_secs(5);
const MEMBER_PREFIX: &str = "member.";
const PART_PREFIX: &str = "part.";

struct OwnedPartition {
    revision: u64,
    shutdown: watch::Sender<bool>,
    handles: Vec<actix_web::rt::task::JoinHandle<()>>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_partitioned<D: RecipientDirectory + RealtimeProjection + 'static, P: RealtimePublisher + 'static, B: RelationDirectory + 'static>(
    jetstream: Context,
    cfg: DispatcherSettings,
    subject_prefix: String,
    partitions: u32,
    replicas: usize,
    dispatch: Arc<DispatchEvent<D, P, B>>,
    mut shutdown: watch::Receiver<bool>,
    liveness: Liveness,
    stale_after: Duration,
) {
    let instance = uuid::Uuid::new_v4().to_string();
    let heartbeat = liveness.register(stale_after);
    let beat = || heartbeat.beat();
    let store = loop {
        match ensure_bucket(&jetstream, replicas).await {
            Ok(store) => break store,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to open partition lease bucket; retrying");
            }
        }
        beat();
        tokio::select! {
            _ = shutdown.changed() => {
                liveness.release(&heartbeat);
                return;
            }
            _ = heartbeat.keep_alive_for(TICK) => {}
        }
        if *shutdown.borrow() {
            liveness.release(&heartbeat);
            return;
        }
    };
    tracing::info!(%instance, partitions, "Partition manager started");

    let mut owned: HashMap<u32, OwnedPartition> = HashMap::new();
    loop {
        beat();
        tick(
            &jetstream,
            &store,
            &cfg,
            &subject_prefix,
            partitions,
            &instance,
            &dispatch,
            &mut owned,
            &liveness,
            stale_after,
        )
        .await;

        tokio::select! {
            _ = shutdown.changed() => break,
            _ = heartbeat.keep_alive_for(TICK) => {}
        }
        if *shutdown.borrow() {
            break;
        }
    }

    let draining: Vec<(u32, OwnedPartition)> = owned.drain().collect();
    for (_, state) in &draining {
        let _ = state.shutdown.send(true);
    }
    futures::future::join_all(
        draining
            .into_iter()
            .map(|(partition, state)| release(&store, partition, state, true)),
    )
    .await;
    let _ = store.purge(format!("{MEMBER_PREFIX}{instance}")).await;
    liveness.release(&heartbeat);
    tracing::info!(%instance, "Partition manager stopped");
}

async fn ensure_bucket(
    jetstream: &Context,
    replicas: usize,
) -> Result<kv::Store, async_nats::Error> {
    let config = kv::Config {
        bucket: BUCKET.to_string(),
        max_age: LEASE_TTL,
        history: 1,
        num_replicas: replicas,
        ..Default::default()
    };
    match jetstream.create_key_value(config).await {
        Ok(store) => Ok(store),
        Err(_) => Ok(jetstream.get_key_value(BUCKET).await?),
    }
}

#[allow(clippy::too_many_arguments)]
async fn tick<D: RecipientDirectory + RealtimeProjection + 'static, P: RealtimePublisher + 'static, B: RelationDirectory + 'static>(
    jetstream: &Context,
    store: &kv::Store,
    cfg: &DispatcherSettings,
    subject_prefix: &str,
    partitions: u32,
    instance: &str,
    dispatch: &Arc<DispatchEvent<D, P, B>>,
    owned: &mut HashMap<u32, OwnedPartition>,
    liveness: &Liveness,
    stale_after: Duration,
) {
    if let Err(e) = store
        .put(format!("{MEMBER_PREFIX}{instance}"), instance.to_string().into_bytes().into())
        .await
    {
        tracing::warn!(error = %e, "Member heartbeat failed");
        return;
    }

    let members = match list_members(store).await {
        Ok(mut members) => {
            if !members.contains(&instance.to_string()) {
                members.push(instance.to_string());
            }
            members.sort();
            members
        }
        Err(e) => {
            tracing::warn!(error = %e, "Member listing failed; keeping current assignment");
            return;
        }
    };

    let assigned = |partition: u32| -> bool {
        members[partition as usize % members.len()] == instance
    };

    for partition in 0..partitions {
        let key = format!("{PART_PREFIX}{partition}");
        if let Some(state) = owned.get_mut(&partition) {
            match store
                .update(&key, instance.to_string().into_bytes().into(), state.revision)
                .await
            {
                Ok(revision) => state.revision = revision,
                Err(e) => {
                    tracing::warn!(partition, error = %e, "Lease renewal failed; releasing partition");
                    if let Some(state) = owned.remove(&partition) {
                        drain_in_background(store, partition, state, false);
                    }
                }
            }
            continue;
        }
        if !assigned(partition) {
            continue;
        }
        if let Ok(revision) = store
            .create(&key, instance.to_string().into_bytes().into())
            .await
        {
            tracing::info!(partition, "Partition acquired");
            let state = start_partition(
                jetstream, cfg, subject_prefix, partition, revision, dispatch,
                liveness,
                stale_after,
            );
            owned.insert(partition, state);
        }
    }

    let released: Vec<u32> = owned
        .keys()
        .copied()
        .filter(|p| !assigned(*p))
        .collect();
    for partition in released {
        if let Some(state) = owned.remove(&partition) {
            tracing::info!(partition, "Releasing partition after rebalance");
            drain_in_background(store, partition, state, true);
        }
    }
}

async fn list_members(store: &kv::Store) -> Result<Vec<String>, async_nats::Error> {
    let mut keys = store.keys().await?;
    let mut members = Vec::new();
    while let Some(key) = keys.next().await {
        let key = key?;
        if let Some(id) = key.strip_prefix(MEMBER_PREFIX) {
            members.push(id.to_string());
        }
    }
    Ok(members)
}

#[allow(clippy::too_many_arguments)]
fn start_partition<D: RecipientDirectory + RealtimeProjection + 'static, P: RealtimePublisher + 'static, B: RelationDirectory + 'static>(
    jetstream: &Context,
    cfg: &DispatcherSettings,
    subject_prefix: &str,
    partition: u32,
    revision: u64,
    dispatch: &Arc<DispatchEvent<D, P, B>>,
    liveness: &Liveness,
    stale_after: Duration,
) -> OwnedPartition {
    let (tx, rx) = watch::channel(false);
    let mut handles = Vec::new();
    let gate = Arc::new(SequenceGate::new(SEQUENCE_WAIT));
    for spec in partition_specs(cfg, subject_prefix, partition) {
        let gate = gate.clone();
        let jetstream = jetstream.clone();
        let dispatch = dispatch.clone();
        let rx = rx.clone();
        let heartbeat = liveness.register(stale_after);
        let liveness = liveness.clone();
        handles.push(actix_web::rt::spawn(async move {
            run_consumer(jetstream, spec, dispatch, rx, heartbeat.clone(), Some(gate)).await;
            liveness.release(&heartbeat);
        }));
    }
    OwnedPartition {
        revision,
        shutdown: tx,
        handles,
    }
}

async fn stop_partition(state: OwnedPartition) {
    let _ = state.shutdown.send(true);
    for handle in state.handles {
        let _ = handle.await;
    }
}

fn drain_in_background(store: &kv::Store, partition: u32, state: OwnedPartition, purge: bool) {
    let store = store.clone();
    actix_web::rt::spawn(async move {
        release(&store, partition, state, purge).await;
    });
}

async fn release(store: &kv::Store, partition: u32, state: OwnedPartition, purge: bool) {
    let revision = state.revision;
    stop_partition(state).await;
    if purge
        && let Err(e) = store
            .purge_expect_revision(format!("{PART_PREFIX}{partition}"), Some(revision))
            .await
    {
        tracing::debug!(partition, error = %e, "Lease purge skipped; key already re-owned or expired");
    }
}

fn partition_specs(
    cfg: &DispatcherSettings,
    subject_prefix: &str,
    partition: u32,
) -> Vec<ConsumerSpec> {
    let prefix = subject_prefix.trim_end_matches('.');
    let consumer_prefix = &cfg.consumer_prefix;
    let max_deliver = cfg.max_deliver;
    let domains = [
        ("user", cfg.user_stream.clone()),
        ("channel", cfg.channel_stream.clone()),
        ("message", cfg.message_stream.clone()),
        ("dm_message", cfg.dm_message_stream.clone()),
        ("community", cfg.community_stream.clone()),
    ];
    domains
        .into_iter()
        .map(|(domain, stream)| ConsumerSpec {
            stream,
            consumer_name: format!("{consumer_prefix}-{domain}-p{partition}"),
            filter_subject: format!("{prefix}.{domain}.p{partition}.>"),
            dlq_subject: format!("{prefix}.dlq.realtime.{domain}.p{partition}"),
            max_deliver,
            start_position: StartPosition::All,
        })
        .collect()
}
