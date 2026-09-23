use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSentCommunityInvitesResponse {
    pub recipient_ids: Vec<Uuid>,
}

impl ListSentCommunityInvitesResponse {
    pub fn new(recipient_ids: Vec<Uuid>) -> Self {
        Self { recipient_ids }
    }
}
