use domain::room::value_objects::token::RoomToken;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRoomResponse {
    pub token: String,
}

impl From<RoomToken> for JoinRoomResponse {
    fn from(value: RoomToken) -> Self {
        Self {
            token: value.as_str().to_owned(),
        }
    }
}
