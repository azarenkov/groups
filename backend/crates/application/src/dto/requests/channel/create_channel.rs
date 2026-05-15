use domain::channel::{kind::ChannelKind, visibility::ChannelVisibility};
use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct CreateChannelRequest {
    pub server_id: Uuid,
    pub name: String,
    pub kind: ChannelKindDto,
    pub visibility: ChannelVisibilityDto,
    pub password: Option<String>,
    pub ctx: RequestContext,
}

impl CreateChannelRequest {
    pub fn new(
        server_id: Uuid,
        name: String,
        kind: ChannelKindDto,
        visibility: ChannelVisibilityDto,
        password: Option<String>,
        ctx: RequestContext,
    ) -> Self {
        Self {
            server_id,
            name,
            kind,
            visibility,
            password,
            ctx,
        }
    }
}

#[derive(Debug)]
pub enum ChannelVisibilityDto {
    Public,
    Private,
}

#[derive(Debug)]
pub enum ChannelKindDto {
    Text,
    Voice,
}

impl From<ChannelVisibilityDto> for ChannelVisibility {
    fn from(value: ChannelVisibilityDto) -> Self {
        match value {
            ChannelVisibilityDto::Public => Self::Public,
            ChannelVisibilityDto::Private => Self::Private,
        }
    }
}

impl From<ChannelKindDto> for ChannelKind {
    fn from(value: ChannelKindDto) -> Self {
        match value {
            ChannelKindDto::Text => Self::Text,
            ChannelKindDto::Voice => Self::Voice,
        }
    }
}
