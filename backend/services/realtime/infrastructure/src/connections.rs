use std::sync::Arc;

use async_nats::Client;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use futures::StreamExt;
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

use realtime_core::ports::directory::RecipientDirectory;
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
    appear_offline: Arc<DashMap<Uuid, ()>>,
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
        let appear_offline = self.appear_offline.clone();
        let by_user = self.by_user.clone();
        let core = self.core.clone();
        let presence_subject = self.presence_subject.clone();
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
                let was_offline = appear_offline.contains_key(&body.user_id);
                if body.appear_offline {
                    appear_offline.insert(body.user_id, ());
                } else {
                    appear_offline.remove(&body.user_id);
                }
                if was_offline == body.appear_offline {
                    continue;
                }
                let has_active = by_user
                    .get(&body.user_id)
                    .map(|s| !s.conns.is_empty())
                    .unwrap_or(false);
                if !has_active {
                    continue;
                }
                let payload = serde_json::to_string(&PresencePayload {
                    user_id: body.user_id,
                    online: !body.appear_offline,
                })
                .unwrap_or_default();
                if let Err(e) =
                    core.publish(presence_subject.clone(), payload.into()).await
                {
                    tracing::warn!(error = %e, user_id = %body.user_id, "Failed to publish presence after preference change");
                }
            }
        });
        Ok(())
    }

    pub fn has_active_conns(&self, user_id: Uuid) -> bool {
        self.by_user
            .get(&user_id)
            .map(|s| !s.conns.is_empty())
            .unwrap_or(false)
    }

    pub async fn set_appear_offline(&self, user_id: Uuid, appear_offline: bool) {
        let was_offline = self.appear_offline.contains_key(&user_id);
        if appear_offline {
            self.appear_offline.insert(user_id, ());
        } else {
            self.appear_offline.remove(&user_id);
        }
        let state_changed = was_offline != appear_offline;
        if !state_changed {
            return;
        }
        if !self.has_active_conns(user_id) {
            return;
        }
        let online_now = !appear_offline;
        let payload = serde_json::to_string(&PresencePayload {
            user_id,
            online: online_now,
        })
        .unwrap_or_default();
        if let Err(e) = self
            .core
            .publish(self.presence_subject.clone(), payload.into())
            .await
        {
            tracing::warn!(error = %e, %user_id, "Failed to publish presence preference change");
        }
    }

    pub async fn start_presence_subscription(&self) -> Result<(), async_nats::SubscribeError> {
        let mut sub = self.core.subscribe(self.presence_subject.clone()).await?;
        let by_user = self.by_user.clone();
        let online = self.online_snapshot.clone();
        let appear_offline = self.appear_offline.clone();
        let core = self.core.clone();
        let presence_subject = self.presence_subject.clone();
        let resolver = self.presence_resolver.clone();
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
                if payload.is_stale_offline(has_local, appear_offline.contains_key(&payload.user_id)) {
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
                            "Failed to resolve presence recipients; skipping fanout",
                        );
                        continue;
                    }
                };
                for recipient in interested {
                    if let Some(slot) = by_user.get(&recipient) {
                        let snapshot: Vec<Connection> = slot.conns.clone();
                        drop(slot);
                        for c in &snapshot {
                            let _ = c.try_send(&text).await;
                        }
                    }
                }
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
                    let handle = tokio::spawn(fanout_loop(
                        user_id,
                        by_user,
                        core,
                        subject_for_task,
                        subscription,
                        appear_offline,
                        presence_subject,
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

        let interested = match self.presence_resolver.interested_in(user_id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, %user_id, "Failed to resolve presence snapshot recipients");
                Vec::new()
            }
        };
        for other_id in interested {
            if self.online_snapshot.contains_key(&other_id) {
                let env = serde_json::to_string(&PresenceUpdate::online(other_id).envelope())
                    .unwrap_or_default();
                let _ = snapshot_conn.try_send(&env).await;
            }
        }

        if was_first_for_user && !self.appear_offline.contains_key(&user_id) {
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
        if removed_last && !self.appear_offline.contains_key(&user_id) {
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
async fn fanout_loop(
    user_id: Uuid,
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    core: async_nats::Client,
    subject: String,
    initial_sub: async_nats::Subscriber,
    appear_offline: Arc<DashMap<Uuid, ()>>,
    presence_subject: String,
) {
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
                        if !appear_offline.contains_key(&user_id) {
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
