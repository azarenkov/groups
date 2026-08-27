use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InviteCode {
    pub code: String,
    pub server_id: Uuid,
    pub created_by: Uuid,
    pub creator_name: Option<String>,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
