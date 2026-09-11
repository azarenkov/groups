use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendsResponseV1 {
    pub friends: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockRelationResponseV1 {
    pub blocking: bool,
    pub blocked_by: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockRelationsRequestV1 {
    pub ids: Vec<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockRelationsResponseV1 {
    pub blocking: Vec<uuid::Uuid>,
    pub blocked_by: Vec<uuid::Uuid>,
}
