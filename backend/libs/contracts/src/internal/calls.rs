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
    pub callee_invited_by_caller: bool,
    pub callee_pending: bool,
    pub callee_answered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub callee_declined: bool,
}
