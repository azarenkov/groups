use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::{channel::repository::ChannelRepository, channel::repository::VisibleVoiceRoom, server::repository::MutualServer, server::repository::ServerRepository};
use ports::{repository::RepositoryError, user_profile::PublicProfile, user_profile::UserProfileRepository};

#[derive(Debug, thiserror::Error)]
pub enum GetUserProfileError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("User not found")]
    NotFound,
}

pub struct UserProfileView {
    pub profile: PublicProfile,
    pub mutual_servers: Vec<MutualServer>,
    pub voice_room: Option<VisibleVoiceRoom>,
}

pub struct GetUserProfile<P, S, C>
where
    P: UserProfileRepository,
    S: ServerRepository,
    C: ChannelRepository,
{
    profiles: Arc<P>,
    servers: Arc<S>,
    channels: Arc<C>,
}

impl<P, S, C> GetUserProfile<P, S, C>
where
    P: UserProfileRepository,
    S: ServerRepository,
    C: ChannelRepository,
{
    pub fn new(profiles: Arc<P>, servers: Arc<S>, channels: Arc<C>) -> Self {
        Self {
            profiles,
            servers,
            channels,
        }
    }

    pub async fn execute(&self, viewer: UserId, target: UserId) -> Result<UserProfileView, GetUserProfileError> {
        let profile = self
            .profiles
            .get_public_profile(&target)
            .await?
            .ok_or(GetUserProfileError::NotFound)?;
        let (mutual_servers, voice_room) = if profile.active {
            (
                self.servers.find_mutual_servers(&viewer, &target).await?,
                self.channels.find_visible_voice_room(&target, &viewer).await?,
            )
        } else {
            (Vec::new(), None)
        };
        Ok(UserProfileView {
            profile,
            mutual_servers,
            voice_room,
        })
    }
}
