use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSentServerInvitesResponse {
    pub recipient_ids: Vec<Uuid>,
}

impl ListSentServerInvitesResponse {
    pub fn new(recipient_ids: Vec<Uuid>) -> Self {
        Self { recipient_ids }
    }
}
