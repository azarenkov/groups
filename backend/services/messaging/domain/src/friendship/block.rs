use chrono::{DateTime, Utc};
use domain::user::id::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockRelation {
    pub blocking: bool,
    pub blocked_by: bool,
}

impl BlockRelation {
    pub fn any(self) -> bool {
        self.blocking || self.blocked_by
    }
}

#[derive(Debug, Clone)]
pub struct BlockedUserView {
    pub user_id: UserId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Clone)]
pub struct BlockRelations {
    pub blocking: Vec<UserId>,
    pub blocked_by: Vec<UserId>,
}
