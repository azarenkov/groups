use domain::user::User;
use uuid::Uuid;

#[derive(Debug)]
pub struct ChannelParticipantDto {
    pub id: Uuid,
    pub name: String,
}

impl From<User> for ChannelParticipantDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id().uuid(),
            name: user.name().as_str().to_string(),
        }
    }
}
