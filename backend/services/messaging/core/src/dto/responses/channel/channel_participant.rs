use domain::{user::id::UserId, user::name::UserName};
use uuid::Uuid;

#[derive(Debug)]
pub struct ChannelParticipantDto {
    pub id: Uuid,
    pub name: String,
}

impl From<(UserId, UserName)> for ChannelParticipantDto {
    fn from((id, name): (UserId, UserName)) -> Self {
        Self {
            id: id.uuid(),
            name: name.as_str().to_string(),
        }
    }
}
