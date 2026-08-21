use chrono::{DateTime, Utc};

use crate::user::id::UserId;

#[derive(Debug)]
pub struct ParticipantMute {
    pub user_id: UserId,
    pub muted_by: UserId,
    pub muted_at: DateTime<Utc>,
}

impl ParticipantMute {
    pub fn new(user_id: UserId, muted_by: UserId, muted_at: DateTime<Utc>) -> Self {
        Self {
            user_id,
            muted_by,
            muted_at,
        }
    }
}
