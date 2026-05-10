use chrono::{DateTime, Utc};
use domain::channel::{Channel, visibility::ChannelVisibility};
use serde::Serialize;
use uuid::Uuid;

use crate::shared::dto::pagination::PaginationMeta;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListServerChannelsResponse {
    pub channels: Vec<ChannelResponse>,
    pub pagination: PaginationMeta,
}

impl ListServerChannelsResponse {
    pub fn new(channels: Vec<Channel>, pagination: PaginationMeta) -> Self {
        Self {
            channels: channels.into_iter().map(ChannelResponse::from).collect(),
            pagination,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub visibility: ChannelVisibilityResponse,
    pub max_participants: u8,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelVisibilityResponse {
    Public,
    Private,
}

impl From<Channel> for ChannelResponse {
    fn from(value: Channel) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_owned(),
            owner_id: value.owner_id().uuid(),
            visibility: (&value.visibility()).into(),
            max_participants: value.max_participants().value(),
            created_at: value.created_at(),
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
