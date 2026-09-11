use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct SendFriendRequestBody {
    #[validate(length(min = 1, max = 100))]
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct ListFriendsQuery {
    pub status: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}
