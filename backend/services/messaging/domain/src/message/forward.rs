use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardSource {
    Channel { community_id: Uuid, channel_id: Uuid },
    Dm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardInfo {
    pub message_id: Uuid,
    pub author_id: UserId,
    pub author_name: String,
    pub sent_at: DateTime<Utc>,
    pub source: ForwardSource,
}

impl ForwardInfo {
    pub fn to_json(&self) -> Value {
        let source = match &self.source {
            ForwardSource::Channel { community_id, channel_id } => json!({
                "kind": "channel",
                "communityId": community_id,
                "channelId": channel_id,
            }),
            ForwardSource::Dm => json!({ "kind": "dm" }),
        };
        json!({
            "messageId": self.message_id,
            "authorId": self.author_id.uuid(),
            "authorName": self.author_name,
            "sentAt": self.sent_at,
            "source": source,
        })
    }

    pub fn from_json(value: &Value) -> Option<Self> {
        let source = value.get("source")?;
        let source = match source.get("kind")?.as_str()? {
            "channel" => ForwardSource::Channel {
                community_id: Uuid::parse_str(source.get("communityId")?.as_str()?).ok()?,
                channel_id: Uuid::parse_str(source.get("channelId")?.as_str()?).ok()?,
            },
            "dm" => ForwardSource::Dm,
            _ => return None,
        };
        Some(Self {
            message_id: Uuid::parse_str(value.get("messageId")?.as_str()?).ok()?,
            author_id: UserId::new(Uuid::parse_str(value.get("authorId")?.as_str()?).ok()?),
            author_name: value.get("authorName")?.as_str()?.to_string(),
            sent_at: DateTime::parse_from_rfc3339(value.get("sentAt")?.as_str()?)
                .ok()?
                .with_timezone(&Utc),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(source: ForwardSource) -> ForwardInfo {
        ForwardInfo {
            message_id: Uuid::new_v4(),
            author_id: UserId::new(Uuid::new_v4()),
            author_name: "Ann".to_string(),
            sent_at: DateTime::parse_from_rfc3339("2026-09-24T10:00:00Z").unwrap().with_timezone(&Utc),
            source,
        }
    }

    #[test]
    fn round_trips_a_channel_source() {
        let original = info(ForwardSource::Channel {
            community_id: Uuid::new_v4(),
            channel_id: Uuid::new_v4(),
        });
        assert_eq!(ForwardInfo::from_json(&original.to_json()), Some(original));
    }

    #[test]
    fn round_trips_a_dm_source() {
        let original = info(ForwardSource::Dm);
        assert_eq!(ForwardInfo::from_json(&original.to_json()), Some(original));
    }

    #[test]
    fn rejects_an_unknown_source() {
        let mut value = info(ForwardSource::Dm).to_json();
        value["source"]["kind"] = json!("voice");
        assert_eq!(ForwardInfo::from_json(&value), None);
    }
}
