use domain::channel::{kind::ChannelKind, visibility::ChannelVisibility};
use uuid::Uuid;

#[derive(Debug)]
pub struct CreateChannelRequest {
    pub server_id: Uuid,
    pub name: String,
    pub kind: ChannelKindDto,
    pub visibility: ChannelVisibilityDto,
    pub password: Option<String>,
}

impl CreateChannelRequest {
    pub fn new(
        server_id: Uuid,
        name: String,
        kind: ChannelKindDto,
        visibility: ChannelVisibilityDto,
        password: Option<String>,
    ) -> Self {
        Self {
            server_id,
            name,
            kind,
            visibility,
            password,
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
    Hybrid,
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
            ChannelKindDto::Hybrid => Self::Hybrid,
        }
    }
}
