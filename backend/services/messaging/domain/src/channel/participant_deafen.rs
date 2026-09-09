use chrono::{DateTime, Utc};

use domain::user::id::UserId;

#[derive(Debug)]
pub struct ParticipantDeafen {
    pub user_id: UserId,
    pub deafened_by: UserId,
    pub deafened_at: DateTime<Utc>,
}

impl ParticipantDeafen {
    pub fn new(user_id: UserId, deafened_by: UserId, deafened_at: DateTime<Utc>) -> Self {
        Self {
            user_id,
            deafened_by,
            deafened_at,
        }
    }
}
