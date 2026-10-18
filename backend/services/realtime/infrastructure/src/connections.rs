use std::sync::Arc;

use async_nats::Client;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use futures::StreamExt;
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;
use ports::repository::RepositoryError;

use realtime_core::ports::directory::{PresencePreference, RecipientDirectory};
use realtime_domain::presence::presence_after_preference;
use realtime_domain::presence::PresenceUpdate;

#[derive(Clone)]
pub struct Connection {
    pub id: Uuid,
    pub user_id: Uuid,
    tx: mpsc::Sender<String>,
}

impl Connection {
    pub fn new(user_id: Uuid, tx: mpsc::Sender<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            tx,
        }
    }

    pub async fn try_send(&self, payload: &str) -> bool {
        self.tx.try_send(payload.to_string()).is_ok()
    }
}

struct UserSlot {
    conns: Vec<Connection>,
    sub_task: JoinHandle<()>,
}

pub struct ConnectionManager<D: RecipientDirectory> {
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    core: Client,
    user_subject_prefix: String,
    presence_subject: String,
    presence_preference_subject: String,
    online_snapshot: Arc<DashMap<Uuid, ()>>,
    appear_offline: Arc<DashMap<Uuid, PreferenceState>>,
    presence_resolver: Arc<D>,
}

impl<D: RecipientDirectory> Clone for ConnectionManager<D> {
    fn clone(&self) -> Self {
        Self {
            by_user: self.by_user.clone(),
            core: self.core.clone(),
            user_subject_prefix: self.user_subject_prefix.clone(),
            presence_subject: self.presence_subject.clone(),
            presence_preference_subject: self.presence_preference_subject.clone(),
            online_snapshot: self.online_snapshot.clone(),
            appear_offline: self.appear_offline.clone(),
            presence_resolver: self.presence_resolver.clone(),
        }
    }
}

type PresencePayload = PresenceUpdate;

use contracts::realtime::PresencePreferenceV1 as PresencePreferenceMessage;

#[derive(Debug, Clone, Copy)]
pub struct PreferenceState {
    hidden: bool,
    as_of: chrono::DateTime<chrono::Utc>,
    unannounced: Option<bool>,
}

fn hidden_by_preference(cache: &DashMap<Uuid, PreferenceState>, user_id: Uuid) -> bool {
    cache.get(&user_id).is_some_and(|state| state.hidden)
}

fn remember_preference(cache: &DashMap<Uuid, PreferenceState>, user_id: Uuid, hidden: bool, as_of: chrono::DateTime<chrono::Utc>) -> Option<Option<bool>> {
    match cache.entry(user_id) {
        Entry::Occupied(mut entry) => {
            if entry.get().as_of > as_of {
                return None;
            }
            let was_hidden = entry.get().hidden;
            *entry.get_mut() = PreferenceState { hidden, as_of, unannounced: None };
            Some(Some(was_hidden))
        }
        Entry::Vacant(slot) => {
            slot.insert(PreferenceState { hidden, as_of, unannounced: None });
            Some(None)
        }
    }
}

const PREFERENCE_RECONCILE_EVERY: std::time::Duration = std::time::Duration::from_secs(30);
const PREFERENCE_RECONCILE_BATCH: usize = 500;

struct PreferenceSync<D: RecipientDirectory> {
    resolver: Arc<D>,
    cache: Arc<DashMap<Uuid, PreferenceState>>,
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    core: async_nats::Client,
    presence_subject: String,
}

impl<D: RecipientDirectory> Clone for PreferenceSync<D> {
    fn clone(&self) -> Self {
        Self {
            resolver: self.resolver.clone(),
            cache: self.cache.clone(),
            by_user: self.by_user.clone(),
            core: self.core.clone(),
            presence_subject: self.presence_subject.clone(),
        }
    }
}

impl<D: RecipientDirectory> PreferenceSync<D> {
    fn has_local(&self, user_id: Uuid) -> bool {
        self.by_user.get(&user_id).map(|slot| !slot.conns.is_empty()).unwrap_or(false)
    }

    fn known_visible(&self, user_id: Uuid) -> bool {
        self.cache.get(&user_id).is_some_and(|state| !state.hidden)
    }

    async fn announce(&self, user_id: Uuid, online: bool) -> bool {
        let payload = serde_json::to_string(&PresencePayload { user_id, online }).unwrap_or_default();
        match self.core.publish(self.presence_subject.clone(), payload.into()).await {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!(error = %e, %user_id, online, "Failed to publish presence after preference change; will retry on reconciliation");
                false
            }
        }
    }

    fn mark_unannounced(&self, user_id: Uuid, pending: Option<bool>) {
        if let Some(mut state) = self.cache.get_mut(&user_id) {
            state.unannounced = pending;
        }
    }

    async fn observe(&self, user_id: Uuid, hidden: bool, as_of: chrono::DateTime<chrono::Utc>) {
        let Some(previously) = remember_preference(&self.cache, user_id, hidden, as_of) else {
            return;
        };
        if let Some(online) = presence_after_preference(previously, hidden)
            && self.has_local(user_id)
        {
            let delivered = self.announce(user_id, online).await;
            self.mark_unannounced(user_id, (!delivered).then_some(online));
        }
    }

    async fn retry_unannounced(&self) {
        let pending: Vec<(Uuid, bool)> = self
            .cache
            .iter()
            .filter_map(|entry| entry.value().unannounced.map(|online| (*entry.key(), online)))
            .collect();
        for (user_id, online) in pending {
            if !self.has_local(user_id) || self.announce(user_id, online).await {
                self.mark_unannounced(user_id, None);
            }
        }
    }

    async fn load(&self, user_ids: &[Uuid]) -> Result<Vec<(Uuid, bool, chrono::DateTime<chrono::Utc>)>, RepositoryError> {
        let found: std::collections::HashMap<Uuid, PresencePreference> =
            self.resolver.presence_preferences(user_ids).await?.into_iter().collect();
        Ok(user_ids
            .iter()
            .map(|user_id| match found.get(user_id) {
                Some(preference) => (*user_id, preference.appear_offline, preference.as_of),
                None => (*user_id, false, chrono::DateTime::<chrono::Utc>::UNIX_EPOCH),
            })
            .collect())
    }

    async fn refresh(&self, user_ids: &[Uuid]) -> Result<(), RepositoryError> {
        for (user_id, hidden, as_of) in self.load(user_ids).await? {
            remember_preference(&self.cache, user_id, hidden, as_of);
        }
        Ok(())
    }

    async fn reconcile(&self, user_ids: &[Uuid]) -> Result<(), RepositoryError> {
        for (user_id, hidden, as_of) in self.load(user_ids).await? {
            self.observe(user_id, hidden, as_of).await;
        }
        self.retry_unannounced().await;
        Ok(())
    }
}

async fn retry_presence_preference<D: RecipientDirectory>(sync: PreferenceSync<D>, user_id: Uuid) {
    for attempt in 0..PRESENCE_RETRY_ATTEMPTS {
        tokio::time::sleep(presence_retry_delay(attempt)).await;
        if !sync.has_local(user_id) {
            return;
        }
        match sync.refresh(&[user_id]).await {
            Ok(()) => {
                if sync.known_visible(user_id) && sync.has_local(user_id) {
                    sync.announce(user_id, true).await;
                }
                return;
            }
            Err(e) => {
                tracing::warn!(error = %e, %user_id, attempt, "Presence preference retry failed");
            }
        }
    }
    tracing::error!(%user_id, "Gave up loading the presence preference; the user stays unannounced until the next reconciliation");
}

impl<D: RecipientDirectory + 'static> ConnectionManager<D> {
    pub fn new(
        core: Client,
        user_subject_prefix: String,
        presence_subject: String,
        presence_preference_subject: String,
        presence_resolver: Arc<D>,
    ) -> Self {
        Self {
            by_user: Arc::new(DashMap::new()),
            core,
            user_subject_prefix,
            presence_subject,
            presence_preference_subject,
            online_snapshot: Arc::new(DashMap::new()),
            appear_offline: Arc::new(DashMap::new()),
            presence_resolver,
        }
    }

    pub async fn start_presence_preference_subscription(
        &self,
    ) -> Result<(), async_nats::SubscribeError> {
        let mut sub = self
            .core
            .subscribe(self.presence_preference_subject.clone())
            .await?;
        let sync = self.preference_sync();
        tokio::spawn(async move {
            while let Some(msg) = sub.next().await {
                let body: PresencePreferenceMessage =
                    match serde_json::from_slice(&msg.payload) {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::warn!(error = %e, "Bad presence preference payload");
                            continue;
                        }
                    };
                if !sync.has_local(body.user_id) {
                    continue;
                }
                let as_of = body.occurred_at.unwrap_or_else(chrono::Utc::now);
                sync.observe(body.user_id, body.appear_offline, as_of).await;
            }
        });
        Ok(())
    }

    pub fn start_preference_reconciliation(&self) {
        let sync = self.preference_sync();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(PREFERENCE_RECONCILE_EVERY).await;
                let users: Vec<Uuid> = sync.by_user.iter().map(|slot| *slot.key()).collect();
                for batch in users.chunks(PREFERENCE_RECONCILE_BATCH) {
                    if let Err(e) = sync.reconcile(batch).await {
                        tracing::warn!(error = %e, users = batch.len(), "Presence preference reconciliation failed");
                    }
                }
            }
        });
    }

    fn preference_sync(&self) -> PreferenceSync<D> {
        PreferenceSync {
            resolver: self.presence_resolver.clone(),
            cache: self.appear_offline.clone(),
            by_user: self.by_user.clone(),
            core: self.core.clone(),
            presence_subject: self.presence_subject.clone(),
        }
    }

    pub async fn visible_online(&self, viewer: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, RepositoryError> {
        let candidates: Vec<Uuid> = ids
            .iter()
            .copied()
            .filter(|id| self.online_snapshot.contains_key(id))
            .collect();
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let audience = self.presence_resolver.interested_in(viewer).await?;
        Ok(candidates
            .into_iter()
            .filter(|id| audience.contains(id))
            .collect())
    }

    pub fn has_active_conns(&self, user_id: Uuid) -> bool {
        self.by_user
            .get(&user_id)
            .map(|s| !s.conns.is_empty())
            .unwrap_or(false)
    }

    pub async fn start_presence_subscription(&self) -> Result<(), async_nats::SubscribeError> {
        let mut sub = self.core.subscribe(self.presence_subject.clone()).await?;
        let by_user = self.by_user.clone();
        let online = self.online_snapshot.clone();
        let appear_offline = self.appear_offline.clone();
        let core = self.core.clone();
        let presence_subject = self.presence_subject.clone();
        let resolver = self.presence_resolver.clone();
        let sync = self.preference_sync();
        tokio::spawn(async move {
            while let Some(msg) = sub.next().await {
                let payload: PresencePayload = match serde_json::from_slice(&msg.payload) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(error = %e, "Bad presence payload");
                        continue;
                    }
                };
                // Multi-replica guard: another replica may have published `offline`
                // because its last local connection for this user dropped, but THIS
                // replica still has active connections. Counter-publish online and
                // skip propagating the stale offline to local clients.
                let has_local = by_user
                    .get(&payload.user_id)
                    .map(|s| !s.conns.is_empty())
                    .unwrap_or(false);
                let preference_verified = if payload.online || !has_local {
                    true
                } else {
                    match sync.refresh(&[payload.user_id]).await {
                        Ok(()) => true,
                        Err(e) => {
                            tracing::warn!(error = %e, user_id = %payload.user_id, "Could not verify the presence preference; leaving the offline update uncountered");
                            false
                        }
                    }
                };
                if preference_verified && payload.is_stale_offline(has_local, hidden_by_preference(&appear_offline, payload.user_id)) {
                    let counter = serde_json::to_string(&PresencePayload {
                        user_id: payload.user_id,
                        online: true,
                    })
                    .unwrap_or_default();
                    if let Err(e) =
                        core.publish(presence_subject.clone(), counter.into()).await
                    {
                        tracing::warn!(
                            error = %e,
                            user_id = %payload.user_id,
                            "Failed to counter-publish presence online",
                        );
                    }
                    continue;
                }

                if payload.online {
                    online.insert(payload.user_id, ());
                } else {
                    online.remove(&payload.user_id);
                }
                let envelope = serde_json::to_string(&payload.envelope());
                let Ok(text) = envelope else {
                    continue;
                };
                let interested = match resolver.interested_in(payload.user_id).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            user_id = %payload.user_id,
                            "Failed to resolve presence recipients; retrying in the background",
                        );
                        tokio::spawn(retry_presence_fanout(
                            resolver.clone(),
                            by_user.clone(),
                            online.clone(),
                            payload.user_id,
                            payload.online,
                            text,
                        ));
                        continue;
                    }
                };
                deliver_to(&by_user, &interested, &text).await;
            }
        });
        Ok(())
    }

    pub async fn register(&self, conn: Connection) -> Result<(), RegisterError> {
        let user_id = conn.user_id;
        let snapshot_conn = conn.clone();

        let was_first_for_user = if let Some(mut entry) = self.by_user.get_mut(&user_id) {
            entry.conns.push(conn);
            false
        } else {
            let subject = format!("{}.{}", self.user_subject_prefix, user_id);
            let subscription = self
                .core
                .subscribe(subject.clone())
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, %user_id, %subject, "Failed to subscribe to user subject");
                    RegisterError::Subscribe(e)
                })?;
            if let Err(e) = self.core.flush().await {
                tracing::warn!(
                    error = %e,
                    %user_id,
                    %subject,
                    "Failed to flush after subscribe; first messages may be dropped",
                );
            }

            match self.by_user.entry(user_id) {
                Entry::Vacant(v) => {
                    let by_user = self.by_user.clone();
                    let core = self.core.clone();
                    let subject_for_task = subject.clone();
                    let appear_offline = self.appear_offline.clone();
                    let presence_subject = self.presence_subject.clone();
                    let online_snapshot = self.online_snapshot.clone();
                    let resolver = self.presence_resolver.clone();
                    let handle = tokio::spawn(fanout_loop(
                        user_id,
                        subject_for_task,
                        subscription,
                        FanoutContext {
                            by_user,
                            core,
                            appear_offline,
                            presence_subject,
                            online_snapshot,
                            resolver,
                        },
                    ));
                    v.insert(UserSlot {
                        conns: vec![conn],
                        sub_task: handle,
                    });
                    true
                }
                Entry::Occupied(mut o) => {
                    o.get_mut().conns.push(conn);
                    false
                }
            }
        };

        let mut preference_known = true;
        if was_first_for_user {
            let sync = self.preference_sync();
            if let Err(e) = sync.refresh(&[user_id]).await {
                preference_known = self.appear_offline.contains_key(&user_id);
                tracing::warn!(error = %e, %user_id, "Failed to load the presence preference; deferring the online announcement");
                if !preference_known {
                    tokio::spawn(retry_presence_preference(sync, user_id));
                }
            }
        }

        match self.presence_resolver.interested_in(user_id).await {
            Ok(interested) => {
                send_online_snapshot(&self.online_snapshot, &interested, &snapshot_conn).await;
            }
            Err(e) => {
                tracing::warn!(error = %e, %user_id, "Failed to resolve presence snapshot recipients; retrying in the background");
                tokio::spawn(retry_presence_snapshot(
                    self.presence_resolver.clone(),
                    self.by_user.clone(),
                    self.online_snapshot.clone(),
                    snapshot_conn,
                ));
            }
        }

        if was_first_for_user && preference_known && !hidden_by_preference(&self.appear_offline, user_id) {
            let payload = serde_json::to_string(&PresencePayload {
                user_id,
                online: true,
            })
            .unwrap_or_default();
            if let Err(e) = self
                .core
                .publish(self.presence_subject.clone(), payload.into())
                .await
            {
                tracing::warn!(error = %e, %user_id, "Failed to publish presence online");
            }
        }

        Ok(())
    }

    pub async fn unregister(&self, user_id: Uuid, conn_id: Uuid) {
        let removed_last = {
            let Entry::Occupied(mut o) = self.by_user.entry(user_id) else {
                return;
            };
            o.get_mut().conns.retain(|c| c.id != conn_id);
            if o.get().conns.is_empty() {
                let slot = o.remove();
                slot.sub_task.abort();
                true
            } else {
                false
            }
        };
        if !removed_last {
            return;
        }
        let hidden_and_announced = self
            .appear_offline
            .remove(&user_id)
            .is_some_and(|(_, state)| state.hidden && state.unannounced.is_none());
        if !hidden_and_announced {
            let payload = serde_json::to_string(&PresencePayload {
                user_id,
                online: false,
            })
            .unwrap_or_default();
            if let Err(e) = self
                .core
                .publish(self.presence_subject.clone(), payload.into())
                .await
            {
                tracing::warn!(error = %e, %user_id, "Failed to publish presence offline");
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
const PRESENCE_RETRY_ATTEMPTS: u32 = 8;

fn presence_retry_delay(attempt: u32) -> std::time::Duration {
    std::time::Duration::from_secs((1u64 << attempt.min(5)).min(30))
}

async fn deliver_to(by_user: &DashMap<Uuid, UserSlot>, recipients: &[Uuid], text: &str) {
    for recipient in recipients {
        if let Some(slot) = by_user.get(recipient) {
            let snapshot: Vec<Connection> = slot.conns.clone();
            drop(slot);
            for c in &snapshot {
                let _ = c.try_send(text).await;
            }
        }
    }
}

async fn send_online_snapshot(online: &DashMap<Uuid, ()>, interested: &[Uuid], conn: &Connection) {
    for other_id in interested {
        if online.contains_key(other_id) {
            let env = serde_json::to_string(&PresenceUpdate::online(*other_id).envelope())
                .unwrap_or_default();
            let _ = conn.try_send(&env).await;
        }
    }
}

async fn retry_presence_fanout<D: RecipientDirectory>(
    resolver: Arc<D>,
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    online: Arc<DashMap<Uuid, ()>>,
    user_id: Uuid,
    was_online: bool,
    text: String,
) {
    for attempt in 0..PRESENCE_RETRY_ATTEMPTS {
        tokio::time::sleep(presence_retry_delay(attempt)).await;
        if online.contains_key(&user_id) != was_online {
            return;
        }
        match resolver.interested_in(user_id).await {
            Ok(interested) => {
                deliver_to(&by_user, &interested, &text).await;
                return;
            }
            Err(e) => {
                tracing::warn!(error = %e, %user_id, attempt, "Presence fanout retry failed");
            }
        }
    }
    tracing::error!(%user_id, "Gave up delivering a presence update after repeated lookup failures");
}

async fn retry_presence_snapshot<D: RecipientDirectory>(
    resolver: Arc<D>,
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    online: Arc<DashMap<Uuid, ()>>,
    conn: Connection,
) {
    for attempt in 0..PRESENCE_RETRY_ATTEMPTS {
        tokio::time::sleep(presence_retry_delay(attempt)).await;
        let still_registered = by_user
            .get(&conn.user_id)
            .map(|slot| slot.conns.iter().any(|c| c.id == conn.id))
            .unwrap_or(false);
        if !still_registered {
            return;
        }
        match resolver.interested_in(conn.user_id).await {
            Ok(interested) => {
                send_online_snapshot(&online, &interested, &conn).await;
                return;
            }
            Err(e) => {
                tracing::warn!(error = %e, user_id = %conn.user_id, attempt, "Presence snapshot retry failed");
            }
        }
    }
    tracing::error!(user_id = %conn.user_id, "Gave up sending a presence snapshot after repeated lookup failures");
}

async fn retry_unblock_presence<D: RecipientDirectory>(
    resolver: Arc<D>,
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    online: Arc<DashMap<Uuid, ()>>,
    user_id: Uuid,
    peer: Uuid,
) {
    for attempt in 0..PRESENCE_RETRY_ATTEMPTS {
        tokio::time::sleep(presence_retry_delay(attempt)).await;
        if !online.contains_key(&peer) {
            return;
        }
        match resolver.interested_in(peer).await {
            Ok(audience) => {
                if audience.contains(&user_id) {
                    let text = serde_json::to_string(&PresenceUpdate::online(peer).envelope())
                        .unwrap_or_default();
                    deliver_to(&by_user, &[user_id], &text).await;
                }
                return;
            }
            Err(e) => {
                tracing::warn!(error = %e, %user_id, %peer, attempt, "Presence restore after unblock failed");
            }
        }
    }
    tracing::error!(%user_id, %peer, "Gave up restoring presence after an unblock");
}

async fn block_presence_update<D: RecipientDirectory + 'static>(
    user_id: Uuid,
    payload: &str,
    online: &Arc<DashMap<Uuid, ()>>,
    by_user: &Arc<DashMap<Uuid, UserSlot>>,
    resolver: &Arc<D>,
) -> Option<PresenceUpdate> {
    if !payload.contains("\"user.blocked\"") && !payload.contains("\"user.unblocked\"") {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    let kind = value.get("type")?.as_str()?;
    let data = value.get("data")?;
    let id_of = |key: &str| {
        data.get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
    };
    let blocker = id_of("blockerId")?;
    let blocked = id_of("blockedId")?;
    let peer = if blocker == user_id { blocked } else { blocker };
    match kind {
        "user.blocked" => Some(PresenceUpdate::offline(peer)),
        "user.unblocked" if online.contains_key(&peer) => match resolver.interested_in(peer).await {
            Ok(audience) => audience.contains(&user_id).then(|| PresenceUpdate::online(peer)),
            Err(e) => {
                tracing::warn!(error = %e, %user_id, %peer, "Presence visibility lookup failed after unblock; retrying in the background");
                tokio::spawn(retry_unblock_presence(
                    resolver.clone(),
                    by_user.clone(),
                    online.clone(),
                    user_id,
                    peer,
                ));
                None
            }
        },
        _ => None,
    }
}

struct FanoutContext<D: RecipientDirectory> {
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    core: async_nats::Client,
    appear_offline: Arc<DashMap<Uuid, PreferenceState>>,
    presence_subject: String,
    online_snapshot: Arc<DashMap<Uuid, ()>>,
    resolver: Arc<D>,
}

async fn fanout_loop<D: RecipientDirectory + 'static>(
    user_id: Uuid,
    subject: String,
    initial_sub: async_nats::Subscriber,
    ctx: FanoutContext<D>,
) {
    let FanoutContext {
        by_user,
        core,
        appear_offline,
        presence_subject,
        online_snapshot,
        resolver,
    } = ctx;
    let mut sub = initial_sub;
    loop {
        while let Some(msg) = sub.next().await {
            let payload = match std::str::from_utf8(&msg.payload) {
                Ok(s) => s.to_string(),
                Err(_) => continue,
            };
            let Some(slot) = by_user.get(&user_id) else {
                return;
            };
            let snapshot: Vec<Connection> = slot.conns.clone();
            drop(slot);
            for c in &snapshot {
                if !c.try_send(&payload).await {
                    tracing::warn!(
                        %user_id,
                        conn_id = %c.id,
                        "Slow consumer outbound buffer full; dropping connection",
                    );
                    let became_empty = if let Some(mut entry) =
                        by_user.get_mut(&user_id)
                    {
                        entry.conns.retain(|x| x.id != c.id);
                        entry.conns.is_empty()
                    } else {
                        false
                    };
                    if became_empty {
                        // Drop the slot but do NOT abort sub_task —
                        // we ARE that task; aborting would cancel us before
                        // the offline publish below completes.
                        let _ = by_user.remove(&user_id);
                        if !hidden_by_preference(&appear_offline, user_id) {
                            let payload = serde_json::to_string(&PresencePayload {
                                user_id,
                                online: false,
                            })
                            .unwrap_or_default();
                            if let Err(e) = core
                                .publish(presence_subject.clone(), payload.into())
                                .await
                            {
                                tracing::warn!(error = %e, %user_id, "Failed to publish presence offline after slow-consumer drop");
                            }
                        }
                        return;
                    }
                }
            }
            if let Some(update) =
                block_presence_update(user_id, &payload, &online_snapshot, &by_user, &resolver).await
            {
                let text = serde_json::to_string(&update.envelope()).unwrap_or_default();
                if let Some(slot) = by_user.get(&user_id) {
                    let conns: Vec<Connection> = slot.conns.clone();
                    drop(slot);
                    for c in &conns {
                        let _ = c.try_send(&text).await;
                    }
                }
            }
        }

        tracing::warn!(%user_id, %subject, "User subject subscription ended; re-subscribing");
        loop {
            if by_user.get(&user_id).is_none() {
                return;
            }
            match core.subscribe(subject.clone()).await {
                Ok(new_sub) => {
                    sub = new_sub;
                    break;
                }
                Err(e) => {
                    tracing::error!(error = %e, %user_id, %subject, "Re-subscribe failed; retrying");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    #[error("nats subscribe: {0}")]
    Subscribe(async_nats::SubscribeError),
}
