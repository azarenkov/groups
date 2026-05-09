use domain::room::room_type::RoomType;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "room_type", rename_all = "lowercase")]
pub enum RoomTypeDto {
    Public,
    Private,
}

impl From<&RoomType> for RoomTypeDto {
    fn from(value: &RoomType) -> Self {
        match value {
            RoomType::Public => Self::Public,
            RoomType::Private => Self::Private,
        }
    }
}

impl From<RoomTypeDto> for RoomType {
    fn from(value: RoomTypeDto) -> Self {
        match value {
            RoomTypeDto::Public => Self::Public,
            RoomTypeDto::Private => Self::Private,
        }
    }
}
