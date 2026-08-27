use chrono::{DateTime, Utc};

use domain::user::id::UserId;

#[derive(Debug)]
pub struct ParticipantMute {
    pub user_id: UserId,
    pub muted_by: UserId,
    pub muted_at: DateTime<Utc>,
    pub self_muted: bool,
}

impl ParticipantMute {
    pub fn new(
        user_id: UserId,
        muted_by: UserId,
        muted_at: DateTime<Utc>,
        self_muted: bool,
    ) -> Self {
        Self {
            user_id,
            muted_by,
            muted_at,
            self_muted,
        }
    }
}
