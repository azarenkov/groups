use std::sync::Arc;

use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc;
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

#[derive(Clone)]
pub struct ConnectionManager {
    by_user: Arc<DashMap<Uuid, Vec<Connection>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            by_user: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, conn: Connection) {
        self.by_user
            .entry(conn.user_id)
            .or_default()
            .push(conn);
    }

    pub fn unregister(&self, user_id: Uuid, conn_id: Uuid) {
        if let Some(mut entry) = self.by_user.get_mut(&user_id) {
            entry.retain(|c| c.id != conn_id);
            if entry.is_empty() {
                drop(entry);
                self.by_user.remove(&user_id);
            }
        }
    }

    pub async fn send_to_users<I, E>(&self, recipients: I, envelope: &E)
    where
        I: IntoIterator<Item = Uuid>,
        E: Serialize,
    {
        let payload = match serde_json::to_string(envelope) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize envelope");
                return;
            }
        };

        let mut delivered_to_users = 0usize;
        let mut total_conns = 0usize;
        let mut dropped = 0usize;

        for user_id in recipients {
            let Some(entry) = self.by_user.get(&user_id) else {
                continue;
            };
            let snapshot: Vec<Connection> = entry.clone();
            drop(entry);

            let mut delivered_here = false;
            for conn in &snapshot {
                total_conns += 1;
                if conn.try_send(&payload).await {
                    delivered_here = true;
                } else {
                    dropped += 1;
                }
            }
            if delivered_here {
                delivered_to_users += 1;
            }
        }

        if dropped > 0 {
            tracing::warn!(
                dropped,
                total_conns,
                users = delivered_to_users,
                "Some realtime sends were dropped (slow client)",
            );
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
