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
    pub next_cursor: Option<String>,
}

impl ListFriendshipsResponse {
    pub fn new(friends: Vec<FriendshipResponse>, next_cursor: Option<String>) -> Self {
        Self {
            friends,
            next_cursor,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendIdItem {
    pub friend_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendIdsResponse {
    pub friends: Vec<FriendIdItem>,
}

impl FriendIdsResponse {
    pub fn new(items: Vec<(Uuid, FriendshipStatus)>) -> Self {
        Self {
            friends: items
                .into_iter()
                .map(|(friend_id, status)| FriendIdItem {
                    friend_id,
                    status: status_str(status).to_string(),
                })
                .collect(),
        }
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
