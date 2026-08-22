use std::collections::HashMap;
use std::time::Duration;

use async_nats::jetstream::{Context, kv};
use futures::StreamExt;
use tokio::sync::watch;

use platform::ports::event_consumer::StartPosition;

use crate::{
    config::DispatcherConfig,
    consumer::{ConsumerSpec, run_consumer},
    publisher::Publisher,
    recipients::RecipientResolver,
};

const BUCKET: &str = "dispatcher-partitions";
const LEASE_TTL: Duration = Duration::from_secs(20);
const TICK: Duration = Duration::from_secs(5);
const MEMBER_PREFIX: &str = "member.";
const PART_PREFIX: &str = "part.";

struct OwnedPartition {
    revision: u64,
    shutdown: watch::Sender<bool>,
    handles: Vec<actix_web::rt::task::JoinHandle<()>>,
}

pub async fn run_partitioned(
    jetstream: Context,
    cfg: DispatcherConfig,
    subject_prefix: String,
    partitions: u32,
    resolver: RecipientResolver,
    publisher: Publisher,
    mut shutdown: watch::Receiver<bool>,
) {
    let instance = uuid::Uuid::new_v4().to_string();
    let store = loop {
        match ensure_bucket(&jetstream).await {
            Ok(store) => break store,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to open partition lease bucket; retrying");
            }
        }
        tokio::select! {
            _ = shutdown.changed() => return,
            _ = tokio::time::sleep(TICK) => {}
        }
        if *shutdown.borrow() {
            return;
        }
    };
    tracing::info!(%instance, partitions, "Partition manager started");

    let mut owned: HashMap<u32, OwnedPartition> = HashMap::new();
    let mut ticks: u64 = 0;
    let mut legacy_maintenance: Option<actix_web::rt::task::JoinHandle<()>> = None;
    loop {
        if ticks.is_multiple_of(6)
            && owned.contains_key(&0)
            && legacy_maintenance
                .as_ref()
                .map(|h| h.is_finished())
                .unwrap_or(true)
        {
            let jetstream = jetstream.clone();
            let cfg = cfg.clone();
            let resolver = resolver.clone();
            let publisher = publisher.clone();
            legacy_maintenance = Some(actix_web::rt::spawn(async move {
                crate::drain_and_remove_legacy(&jetstream, &cfg, &resolver, &publisher).await;
            }));
        }
        ticks += 1;
        tick(
            &jetstream,
            &store,
            &cfg,
            &subject_prefix,
            partitions,
            &instance,
            &resolver,
            &publisher,
            &mut owned,
        )
        .await;

        tokio::select! {
            _ = shutdown.changed() => break,
            _ = tokio::time::sleep(TICK) => {}
        }
        if *shutdown.borrow() {
            break;
        }
    }

    if let Some(handle) = legacy_maintenance.take() {
        handle.abort();
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
    tracing::info!(%instance, "Partition manager stopped");
}

async fn ensure_bucket(jetstream: &Context) -> Result<kv::Store, async_nats::Error> {
    let config = kv::Config {
        bucket: BUCKET.to_string(),
        max_age: LEASE_TTL,
        history: 1,
        ..Default::default()
    };
    match jetstream.create_key_value(config).await {
        Ok(store) => Ok(store),
        Err(_) => Ok(jetstream.get_key_value(BUCKET).await?),
    }
}

#[allow(clippy::too_many_arguments)]
async fn tick(
    jetstream: &Context,
    store: &kv::Store,
    cfg: &DispatcherConfig,
    subject_prefix: &str,
    partitions: u32,
    instance: &str,
    resolver: &RecipientResolver,
    publisher: &Publisher,
    owned: &mut HashMap<u32, OwnedPartition>,
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
                jetstream, cfg, subject_prefix, partition, revision, resolver, publisher,
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

fn start_partition(
    jetstream: &Context,
    cfg: &DispatcherConfig,
    subject_prefix: &str,
    partition: u32,
    revision: u64,
    resolver: &RecipientResolver,
    publisher: &Publisher,
) -> OwnedPartition {
    let (tx, rx) = watch::channel(false);
    let mut handles = Vec::new();
    for spec in partition_specs(cfg, subject_prefix, partition) {
        let jetstream = jetstream.clone();
        let resolver = resolver.clone();
        let publisher = publisher.clone();
        let rx = rx.clone();
        handles.push(actix_web::rt::spawn(async move {
            run_consumer(jetstream, spec, resolver, publisher, rx).await;
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
    cfg: &DispatcherConfig,
    subject_prefix: &str,
    partition: u32,
) -> Vec<ConsumerSpec> {
    let prefix = subject_prefix.trim_end_matches('.');
    let consumer_prefix = &cfg.dispatcher_consumer_prefix;
    let max_deliver = cfg.dispatcher_max_deliver;
    let domains = [
        ("user", cfg.dispatcher_user_stream.clone()),
        ("channel", cfg.dispatcher_channel_stream.clone()),
        ("message", cfg.dispatcher_message_stream.clone()),
        ("dm_message", cfg.dispatcher_dm_message_stream.clone()),
        ("server", cfg.dispatcher_server_stream.clone()),
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
