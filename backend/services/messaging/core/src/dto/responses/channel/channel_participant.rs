use domain::{user::id::UserId, user::name::UserName};
use uuid::Uuid;

#[derive(Debug)]
pub struct ChannelParticipantDto {
    pub id: Uuid,
    pub name: String,
    pub bio: String,
    pub profile_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<(UserId, UserName, String, Option<chrono::DateTime<chrono::Utc>>)> for ChannelParticipantDto {
    fn from((id, name, bio, profile_updated_at): (UserId, UserName, String, Option<chrono::DateTime<chrono::Utc>>)) -> Self {
        Self {
            id: id.uuid(),
            name: name.as_str().to_string(),
            bio,
            profile_updated_at,
        }
    }
}
