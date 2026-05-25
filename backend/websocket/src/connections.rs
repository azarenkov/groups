use std::sync::Arc;

use async_nats::Client;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

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

#[derive(Clone)]
pub struct ConnectionManager {
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    core: Client,
    user_subject_prefix: String,
    presence_subject: String,
    online_snapshot: Arc<DashMap<Uuid, ()>>,
}

#[derive(Serialize, Deserialize)]
struct PresencePayload {
    user_id: Uuid,
    online: bool,
}

impl ConnectionManager {
    pub fn new(
        core: Client,
        user_subject_prefix: String,
        presence_subject: String,
    ) -> Self {
        Self {
            by_user: Arc::new(DashMap::new()),
            core,
            user_subject_prefix,
            presence_subject,
            online_snapshot: Arc::new(DashMap::new()),
        }
    }

    pub async fn start_presence_subscription(&self) -> Result<(), async_nats::SubscribeError> {
        let mut sub = self.core.subscribe(self.presence_subject.clone()).await?;
        let by_user = self.by_user.clone();
        let online = self.online_snapshot.clone();
        tokio::spawn(async move {
            while let Some(msg) = sub.next().await {
                let payload: PresencePayload = match serde_json::from_slice(&msg.payload) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(error = %e, "Bad presence payload");
                        continue;
                    }
                };
                if payload.online {
                    online.insert(payload.user_id, ());
                } else {
                    online.remove(&payload.user_id);
                }
                let envelope = serde_json::to_string(&json!({
                    "type": if payload.online { "user.online" } else { "user.offline" },
                    "data": { "userId": payload.user_id },
                }));
                let Ok(text) = envelope else {
                    continue;
                };
                for entry in by_user.iter() {
                    let snapshot: Vec<Connection> = entry.conns.clone();
                    for c in &snapshot {
                        let _ = c.try_send(&text).await;
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

            match self.by_user.entry(user_id) {
                Entry::Vacant(v) => {
                    let by_user = self.by_user.clone();
                    let core = self.core.clone();
                    let subject_for_task = subject.clone();
                    let handle = tokio::spawn(fanout_loop(
                        user_id,
                        by_user,
                        core,
                        subject_for_task,
                        subscription,
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

        for entry in self.online_snapshot.iter() {
            let other_id = *entry.key();
            let env = serde_json::to_string(&json!({
                "type": "user.online",
                "data": { "userId": other_id },
            }))
            .unwrap_or_default();
            let _ = snapshot_conn.try_send(&env).await;
        }

        if was_first_for_user {
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
        if removed_last {
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

async fn fanout_loop(
    user_id: Uuid,
    by_user: Arc<DashMap<Uuid, UserSlot>>,
    core: async_nats::Client,
    subject: String,
    initial_sub: async_nats::Subscriber,
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
                    if let Some(mut entry) = by_user.get_mut(&user_id) {
                        entry.conns.retain(|x| x.id != c.id);
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
