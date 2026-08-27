use chrono::{DateTime, Utc};

use crate::{channel::id::ChannelId, message::id::MessageId};
use domain::user::id::UserId;

#[derive(Debug, Clone)]
pub struct MessageView {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub author_id: UserId,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}
