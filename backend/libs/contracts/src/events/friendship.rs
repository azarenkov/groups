use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequestSentV1 {
    pub sender_id: Uuid,
    pub receiver_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequestAcceptedV1 {
    pub accepter_id: Uuid,
    pub sender_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendshipRemovedV1 {
    pub user_id: Uuid,
    pub other_id: Uuid,
    pub occurred_at: DateTime<Utc>,
}
