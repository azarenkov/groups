use domain::channel::channel_type::ChannelType;
use uuid::Uuid;

#[derive(Debug)]
pub struct CreateChannelRequest {
    pub server_id: Uuid,
    pub name: String,
    pub channel_type: ChannelTypeDto,
    pub password: Option<String>,
}

#[derive(Debug)]
pub enum ChannelTypeDto {
    Public,
    Private,
}

impl From<ChannelTypeDto> for ChannelType {
    fn from(value: ChannelTypeDto) -> Self {
        match value {
            ChannelTypeDto::Public => Self::Public,
            ChannelTypeDto::Private => Self::Private,
        }
    }
}
