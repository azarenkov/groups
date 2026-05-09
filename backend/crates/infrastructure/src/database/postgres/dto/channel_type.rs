use domain::channel::channel_type::ChannelType;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "channel_type", rename_all = "lowercase")]
pub enum ChannelTypeDto {
    Public,
    Private,
}

impl From<&ChannelType> for ChannelTypeDto {
    fn from(value: &ChannelType) -> Self {
        match value {
            ChannelType::Public => Self::Public,
            ChannelType::Private => Self::Private,
        }
    }
}

impl From<ChannelTypeDto> for ChannelType {
    fn from(value: ChannelTypeDto) -> Self {
        match value {
            ChannelTypeDto::Public => Self::Public,
            ChannelTypeDto::Private => Self::Private,
        }
    }
}
