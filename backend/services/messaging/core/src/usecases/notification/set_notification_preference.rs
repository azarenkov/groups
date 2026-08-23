use std::sync::Arc;

use domain::{
    channel::{id::ChannelId, repository::ChannelRepository},
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    notification::{
        NotificationLevel, NotificationScope,
        repository::{NotificationPreference, NotificationPreferenceRepository},
    },
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum SetNotificationPreferenceError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of this channel")]
    NotMember,
    #[error("Not friends")]
    NotFriends,
    #[error("Level is not supported for this scope")]
    UnsupportedLevel,
}

pub struct SetNotificationPreference<
    R: NotificationPreferenceRepository,
    C: ChannelRepository,
    F: FriendshipRepository,
> {
    repo: Arc<R>,
    channel_repo: Arc<C>,
    friendship_repo: Arc<F>,
}

impl<R: NotificationPreferenceRepository, C: ChannelRepository, F: FriendshipRepository>
    SetNotificationPreference<R, C, F>
{
    pub fn new(repo: Arc<R>, channel_repo: Arc<C>, friendship_repo: Arc<F>) -> Self {
        Self {
            repo,
            channel_repo,
            friendship_repo,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        scope: NotificationScope,
        scope_id: Uuid,
        level: NotificationLevel,
    ) -> Result<i64, SetNotificationPreferenceError> {
        match scope {
            NotificationScope::Channel => {
                let channel_id = ChannelId::new(scope_id);
                let is_member = self
                    .channel_repo
                    .is_text_channel_member(&channel_id, &user_id)
                    .await?;
                if !is_member {
                    return Err(SetNotificationPreferenceError::NotMember);
                }
            }
            NotificationScope::Dm => {
                if level == NotificationLevel::Mentions {
                    return Err(SetNotificationPreferenceError::UnsupportedLevel);
                }
                let friend_id = UserId::new(scope_id);
                let pair = self
                    .friendship_repo
                    .find_pair(&user_id, &friend_id)
                    .await?;
                let accepted = matches!(
                    pair.as_ref().map(|f| &f.status),
                    Some(&FriendshipStatus::Accepted),
                );
                if !accepted {
                    return Err(SetNotificationPreferenceError::NotFriends);
                }
            }
        }
        let version = self
            .repo
            .set_preference(
                &user_id,
                &NotificationPreference {
                    scope,
                    scope_id,
                    level,
                },
            )
            .await?;
        Ok(version)
    }
}
