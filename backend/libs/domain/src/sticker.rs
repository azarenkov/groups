use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickerSnapshot {
    pub id: Uuid,
    pub pack_id: Uuid,
    pub emoji: String,
    pub animated: bool,
    pub width: i32,
    pub height: i32,
}

impl StickerSnapshot {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let sticker = StickerSnapshot {
            id: Uuid::new_v4(),
            pack_id: Uuid::new_v4(),
            emoji: "😺".to_string(),
            animated: true,
            width: 320,
            height: 240,
        };
        let json = sticker.to_json();
        assert_eq!(json["packId"], serde_json::json!(sticker.pack_id));
        assert_eq!(StickerSnapshot::from_json(&json), Some(sticker));
        assert_eq!(StickerSnapshot::from_json(&serde_json::json!({"id": 1})), None);
    }
}
