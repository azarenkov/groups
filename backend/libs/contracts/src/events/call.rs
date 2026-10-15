use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEventV1 {
    pub call_id: Uuid,
    pub actor_id: Uuid,
    pub recipients: Vec<Uuid>,
    #[serde(default)]
    pub video: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invitations: Vec<CallInvitationV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inviter_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallInvitationV1 {
    pub inviter_id: Uuid,
    pub invitee_id: Uuid,
}
