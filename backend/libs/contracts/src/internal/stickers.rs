use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickerInfoV1 {
    pub id: Uuid,
    pub pack_id: Uuid,
    #[serde(default)]
    pub pack_name: String,
    #[serde(default)]
    pub pack_slug: String,
    pub emoji: String,
    pub animated: bool,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickerPackInfoV1 {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
}
