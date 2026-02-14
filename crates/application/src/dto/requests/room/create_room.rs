use domain::entites::room::RoomType;
use uuid::Uuid;

#[derive(Debug)]
pub struct CreateRoomRequest {
    pub server_id: Uuid,
    pub name: String,
    pub room_type: RoomTypeDto,
    pub password: Option<String>,
}

#[derive(Debug)]
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
