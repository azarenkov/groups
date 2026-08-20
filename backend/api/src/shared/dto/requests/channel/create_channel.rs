use serde::Deserialize;

use application::dto::requests::channel::create_channel::{ChannelKindDto, ChannelVisibilityDto};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateChannelRequestBody {
    #[validate(length(min = 3, max = 15))]
    pub name: String,
    pub kind: ChannelKindRequest,
    pub visibility: ChannelVisibilityRequest,
    #[validate(length(min = 4, max = 30))]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelVisibilityRequest {
    Public,
    Private,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKindRequest {
    Text,
    Voice,
}

impl From<ChannelVisibilityRequest> for ChannelVisibilityDto {
    fn from(value: ChannelVisibilityRequest) -> Self {
        match value {
            ChannelVisibilityRequest::Public => Self::Public,
            ChannelVisibilityRequest::Private => Self::Private,
        }
    }
}

impl From<ChannelKindRequest> for ChannelKindDto {
    fn from(value: ChannelKindRequest) -> Self {
        match value {
            ChannelKindRequest::Text => Self::Text,
            ChannelKindRequest::Voice => Self::Voice,
        }
    }
}
