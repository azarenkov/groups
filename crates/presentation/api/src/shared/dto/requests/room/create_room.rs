use serde::Deserialize;

use application::dto::requests::room::create_room::{
    CreateRoomRequest as CreateRoomRequestApplication, RoomTypeDto,
};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateRoomRequest {
    #[validate(length(min = 3, max = 15))]
    pub name: String,
    pub room_type: RoomTypeRequest,
    #[validate(length(min = 4, max = 30))]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomTypeRequest {
    Public,
    Private,
}

impl From<CreateRoomRequest> for CreateRoomRequestApplication {
    fn from(value: CreateRoomRequest) -> Self {
        Self {
            name: value.name,
            room_type: value.room_type.into(),
            password: value.password,
        }
    }
}

impl From<RoomTypeRequest> for RoomTypeDto {
    fn from(value: RoomTypeRequest) -> Self {
        match value {
            RoomTypeRequest::Public => Self::Public,
            RoomTypeRequest::Private => Self::Private,
        }
    }
}
