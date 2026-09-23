use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::{channel::repository::ChannelRepository, channel::repository::VisibleVoiceRoom, community::repository::MutualCommunity, community::repository::CommunityRepository};
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
    pub mutual_communities: Vec<MutualCommunity>,
    pub voice_room: Option<VisibleVoiceRoom>,
}

pub struct GetUserProfile<P, S, C>
where
    P: UserProfileRepository,
    S: CommunityRepository,
    C: ChannelRepository,
{
    profiles: Arc<P>,
    communities: Arc<S>,
    channels: Arc<C>,
}

impl<P, S, C> GetUserProfile<P, S, C>
where
    P: UserProfileRepository,
    S: CommunityRepository,
    C: ChannelRepository,
{
    pub fn new(profiles: Arc<P>, communities: Arc<S>, channels: Arc<C>) -> Self {
        Self {
            profiles,
            communities,
            channels,
        }
    }

    pub async fn execute(&self, viewer: UserId, target: UserId) -> Result<UserProfileView, GetUserProfileError> {
        let profile = self
            .profiles
            .get_public_profile(&target)
            .await?
            .ok_or(GetUserProfileError::NotFound)?;
        let (mutual_communities, voice_room) = if profile.active {
            (
                self.communities.find_mutual_communities(&viewer, &target).await?,
                self.channels.find_visible_voice_room(&target, &viewer).await?,
            )
        } else {
            (Vec::new(), None)
        };
        Ok(UserProfileView {
            profile,
            mutual_communities,
            voice_room,
        })
    }
}
