use domain::entites::room::RoomType;

#[derive(sqlx::Type)]
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
