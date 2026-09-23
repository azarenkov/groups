use ports::outbox::record::AggregateType;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "aggregate_type", rename_all = "snake_case")]
pub enum AggregateTypeDto {
    Channel,
    Community,
    User,
    Message,
    DmMessage,
    Storage,
}

impl From<&AggregateType> for AggregateTypeDto {
    fn from(value: &AggregateType) -> Self {
        match value {
            AggregateType::Channel => Self::Channel,
            AggregateType::Community => Self::Community,
            AggregateType::User => Self::User,
            AggregateType::Message => Self::Message,
            AggregateType::DmMessage => Self::DmMessage,
            AggregateType::Storage => Self::Storage,
        }
    }
}

impl From<AggregateTypeDto> for AggregateType {
    fn from(value: AggregateTypeDto) -> Self {
        match value {
            AggregateTypeDto::Channel => Self::Channel,
            AggregateTypeDto::Community => Self::Community,
            AggregateTypeDto::User => Self::User,
            AggregateTypeDto::Message => Self::Message,
            AggregateTypeDto::DmMessage => Self::DmMessage,
            AggregateTypeDto::Storage => Self::Storage,
        }
    }
}
