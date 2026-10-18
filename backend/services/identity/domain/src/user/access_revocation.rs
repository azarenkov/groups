use chrono::{DateTime, Utc};
use domain::user::id::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRevocation {
    user_id: UserId,
    cutoff: DateTime<Utc>,
}

impl AccessRevocation {
    pub fn new(user_id: UserId, cutoff: DateTime<Utc>) -> Self {
        Self { user_id, cutoff }
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn cutoff(&self) -> DateTime<Utc> {
        self.cutoff
    }
}
