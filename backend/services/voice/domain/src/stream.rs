use chrono::{DateTime, Utc};
use domain::{channel::id::ChannelId, stream::StreamCeiling, user::id::UserId};

#[derive(Debug, Clone, PartialEq)]
pub struct StreamLimited {
    pub user_id: UserId,
    pub room: ChannelId,
    pub track_sid: String,
    pub ceiling: StreamCeiling,
    pub occurred_at: DateTime<Utc>,
}
