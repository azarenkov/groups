use messaging::dto::responses::friendship::friendship::FriendshipDto;
use chrono::{DateTime, Utc};
use messaging_domain::friendship::status::FriendshipStatus;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendshipResponse {
    pub friend_id: Uuid,
    pub friend_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<FriendshipDto> for FriendshipResponse {
    fn from(value: FriendshipDto) -> Self {
        Self {
            friend_id: value.friend_id,
            friend_name: value.friend_name,
            status: status_str(value.status).to_string(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListFriendshipsResponse {
    pub friends: Vec<FriendshipResponse>,
}

impl ListFriendshipsResponse {
    pub fn new(friends: Vec<FriendshipResponse>) -> Self {
        Self { friends }
    }
}

fn status_str(status: FriendshipStatus) -> &'static str {
    match status {
        FriendshipStatus::Accepted => "accepted",
        FriendshipStatus::PendingIncoming => "pending_incoming",
        FriendshipStatus::PendingOutgoing => "pending_outgoing",
        FriendshipStatus::Declined => "declined",
    }
}

pub fn parse_status(s: &str) -> Option<FriendshipStatus> {
    match s {
        "accepted" => Some(FriendshipStatus::Accepted),
        "pending_incoming" => Some(FriendshipStatus::PendingIncoming),
        "pending_outgoing" => Some(FriendshipStatus::PendingOutgoing),
        "declined" => Some(FriendshipStatus::Declined),
        _ => None,
    }
}
