use chrono::{DateTime, Utc};
use domain::channel::{Channel, channel_type::ChannelType};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChannelResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub channel_type: ChannelTypeResponse,
    pub max_participants: u8,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelTypeResponse {
    Public,
    Private,
}

impl From<Channel> for CreateChannelResponse {
    fn from(value: Channel) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_owned(),
            owner_id: value.owner_id().uuid(),
            channel_type: value.channel_type().into(),
            max_participants: value.max_participants().value(),
            created_at: value.created_at(),
        }
    }
}

impl From<&ChannelType> for ChannelTypeResponse {
    fn from(value: &ChannelType) -> Self {
        match value {
            ChannelType::Public => Self::Public,
            ChannelType::Private => Self::Private,
        }
    }
}
