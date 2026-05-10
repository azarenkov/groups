use domain::channel::visibility::ChannelVisibility;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "visibility", rename_all = "lowercase")]
pub enum ChannelVisibilityDto {
    Public,
    Private,
}

impl From<&ChannelVisibility> for ChannelVisibilityDto {
    fn from(value: &ChannelVisibility) -> Self {
        match value {
            ChannelVisibility::Public => Self::Public,
            ChannelVisibility::Private => Self::Private,
        }
    }
}

impl From<ChannelVisibilityDto> for ChannelVisibility {
    fn from(value: ChannelVisibilityDto) -> Self {
        match value {
            ChannelVisibilityDto::Public => Self::Public,
            ChannelVisibilityDto::Private => Self::Private,
        }
    }
}
