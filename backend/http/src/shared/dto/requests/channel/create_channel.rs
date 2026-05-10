use serde::Deserialize;

use application::dto::requests::channel::create_channel::{
    ChannelTypeDto, CreateChannelRequest as CreateChannelRequestApplication,
};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateChannelRequest {
    pub server_id: Uuid,
    #[validate(length(min = 3, max = 15))]
    pub name: String,
    pub channel_type: ChannelTypeRequest,
    #[validate(length(min = 4, max = 30))]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelTypeRequest {
    Public,
    Private,
}

impl From<CreateChannelRequest> for CreateChannelRequestApplication {
    fn from(value: CreateChannelRequest) -> Self {
        Self {
            server_id: value.server_id,
            name: value.name,
            channel_type: value.channel_type.into(),
            password: value.password,
        }
    }
}

impl From<ChannelTypeRequest> for ChannelTypeDto {
    fn from(value: ChannelTypeRequest) -> Self {
        match value {
            ChannelTypeRequest::Public => Self::Public,
            ChannelTypeRequest::Private => Self::Private,
        }
    }
}
