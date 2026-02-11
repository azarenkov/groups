use domain::entites::room::RoomType;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateRoomRequest {
    #[validate(length(min = 3, max = 15))]
    pub name: String,
    pub room_type: RoomTypeDto,
    #[validate(length(min = 4, max = 30))]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub enum RoomTypeDto {
    Public,
    Private,
}

impl From<RoomTypeDto> for RoomType {
    fn from(value: RoomTypeDto) -> Self {
        match value {
            RoomTypeDto::Public => Self::Public,
            RoomTypeDto::Private => Self::Private,
        }
    }
}
