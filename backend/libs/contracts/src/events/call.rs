use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEventV1 {
    pub call_id: Uuid,
    pub actor_id: Uuid,
    pub recipients: Vec<Uuid>,
    #[serde(default)]
    pub video: bool,
}
