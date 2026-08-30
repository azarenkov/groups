use domain::{user::id::UserId, user::name::UserName};
use uuid::Uuid;

#[derive(Debug)]
pub struct ChannelParticipantDto {
    pub id: Uuid,
    pub name: String,
    pub bio: String,
}

impl From<(UserId, UserName, String)> for ChannelParticipantDto {
    fn from((id, name, bio): (UserId, UserName, String)) -> Self {
        Self {
            id: id.uuid(),
            name: name.as_str().to_string(),
            bio,
        }
    }
}
