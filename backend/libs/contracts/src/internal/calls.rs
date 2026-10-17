use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallPermitResponseV1 {
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallOutcomeResponseV1 {
    pub participated: bool,
    pub caller_in_call: bool,
    pub caller_left_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub callee_left_at: Option<chrono::DateTime<chrono::Utc>>,
    pub callee_invited_by_caller: bool,
    pub callee_pending: bool,
    pub callee_answered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub callee_declined: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallInvitationV1 {
    pub inviter_id: uuid::Uuid,
    pub invitee_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndedCallResponseV1 {
    pub ended_at: chrono::DateTime<chrono::Utc>,
    pub invitations: Vec<CallInvitationV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallRoomsRequestV1 {
    pub ids: Vec<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallRoomsResponseV1 {
    pub call_ids: Vec<uuid::Uuid>,
}
