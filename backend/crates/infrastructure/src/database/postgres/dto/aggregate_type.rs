use domain::shared::outbox::outbox_record::AggregateType;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "aggregate_type", rename_all = "snake_case")]
pub enum AggregateTypeDto {
    Channel,
    Server,
    User,
    Message,
    DmMessage,
}

impl From<&AggregateType> for AggregateTypeDto {
    fn from(value: &AggregateType) -> Self {
        match value {
            AggregateType::Channel => Self::Channel,
            AggregateType::Server => Self::Server,
            AggregateType::User => Self::User,
            AggregateType::Message => Self::Message,
            AggregateType::DmMessage => Self::DmMessage,
        }
    }
}

impl From<AggregateTypeDto> for AggregateType {
    fn from(value: AggregateTypeDto) -> Self {
        match value {
            AggregateTypeDto::Channel => Self::Channel,
            AggregateTypeDto::Server => Self::Server,
            AggregateTypeDto::User => Self::User,
            AggregateTypeDto::Message => Self::Message,
            AggregateTypeDto::DmMessage => Self::DmMessage,
        }
    }
}
