use domain::channel::kind::ChannelKind;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "kind", rename_all = "lowercase")]
pub enum ChannelKindDto {
    Text,
    Voice,
    Hybrid,
}

impl From<&ChannelKind> for ChannelKindDto {
    fn from(value: &ChannelKind) -> Self {
        match value {
            ChannelKind::Text => Self::Text,
            ChannelKind::Voice => Self::Voice,
            ChannelKind::Hybrid => Self::Hybrid,
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
