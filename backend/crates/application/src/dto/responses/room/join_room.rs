use domain::room::value_objects::token::RoomToken;

#[derive(Debug)]
pub struct JoinRoomResponse {
    pub token: RoomToken,
    pub url: String,
}

impl JoinRoomResponse {
    pub fn new(token: RoomToken, url: String) -> Self {
        Self { token, url }
    }
}
