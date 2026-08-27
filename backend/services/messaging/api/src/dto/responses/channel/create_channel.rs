use chrono::{DateTime, Utc};
use messaging_domain::channel::{Channel, kind::ChannelKind, visibility::ChannelVisibility};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChannelResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub kind: ChannelKindResponse,
    pub visibility: ChannelVisibilityResponse,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKindResponse {
    Text,
    Voice,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelVisibilityResponse {
    Public,
    Private,
}

impl From<Channel> for CreateChannelResponse {
    fn from(value: Channel) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_owned(),
            owner_id: value.owner_id().uuid(),
            kind: value.kind().into(),
            visibility: (&value.visibility()).into(),
            created_at: value.created_at(),
        }
    }
}

impl From<&ChannelKind> for ChannelKindResponse {
    fn from(value: &ChannelKind) -> Self {
        match value {
            ChannelKind::Text => Self::Text,
            ChannelKind::Voice => Self::Voice,
        }
    }
}

impl From<&ChannelVisibility> for ChannelVisibilityResponse {
    fn from(value: &ChannelVisibility) -> Self {
        match value {
            ChannelVisibility::Public => Self::Public,
            ChannelVisibility::Private => Self::Private,
        }
    }
}
