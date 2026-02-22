use domain::room::value_objects::token::RoomToken;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRoomResponse {
    pub token: String,
    pub url: String,
}

impl JoinRoomResponse {
    pub fn new(token: RoomToken, url: String) -> Self {
        Self {
            token: token.as_str().to_owned(),
            url,
        }
    }
}

impl From<application::dto::responses::room::join_room::JoinRoomResponse> for JoinRoomResponse {
    fn from(value: application::dto::responses::room::join_room::JoinRoomResponse) -> Self {
        Self {
            token: value.token.to_string(),
            url: value.url,
        }
    }
}
