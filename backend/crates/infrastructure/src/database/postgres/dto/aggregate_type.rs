use domain::shared::outbox::outbox_record::AggregateType;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "aggregate_type", rename_all = "snake_case")]
pub enum AggregateTypeDto {
    Channel,
    Server,
    User,
}

impl From<&AggregateType> for AggregateTypeDto {
    fn from(value: &AggregateType) -> Self {
        match value {
            AggregateType::Channel => Self::Channel,
            AggregateType::Server => Self::Server,
            AggregateType::User => Self::User,
        }
    }
}

impl From<AggregateTypeDto> for AggregateType {
    fn from(value: AggregateTypeDto) -> Self {
        match value {
            AggregateTypeDto::Channel => Self::Channel,
            AggregateTypeDto::Server => Self::Server,
            AggregateTypeDto::User => Self::User,
        }
    }
}
