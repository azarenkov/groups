use std::sync::Arc;

use async_nats::Client;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use futures::StreamExt;
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
}

impl ConnectionManager {
    pub fn new(core: Client, user_subject_prefix: String) -> Self {
        Self {
            by_user: Arc::new(DashMap::new()),
            core,
            user_subject_prefix,
        }
    }

    pub async fn register(&self, conn: Connection) -> Result<(), RegisterError> {
        let user_id = conn.user_id;

        if let Some(mut entry) = self.by_user.get_mut(&user_id) {
            entry.conns.push(conn);
            return Ok(());
        }

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
            }
            Entry::Occupied(mut o) => {
                o.get_mut().conns.push(conn);
            }
        }
        Ok(())
    }

    pub async fn unregister(&self, user_id: Uuid, conn_id: Uuid) {
        let Entry::Occupied(mut o) = self.by_user.entry(user_id) else {
            return;
        };
        o.get_mut().conns.retain(|c| c.id != conn_id);
        if o.get().conns.is_empty() {
            let slot = o.remove();
            slot.sub_task.abort();
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
                let _ = c.try_send(&payload).await;
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
