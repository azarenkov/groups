use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, friendship::FriendshipError, friendship::pair::FriendshipPair, friendship::unit_of_work::FriendshipUnitOfWork};
use domain::{user::id::UserId, user::name::UserName};
use ports::repository::RepositoryError;

use crate::dto::requests::friendship::send_friend_request::SendFriendRequestRequest;

#[derive(Debug, thiserror::Error)]
pub enum SendFriendRequestError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Friendship error: {0}")]
    Domain(#[from] FriendshipError),
    #[error("Invalid username")]
    InvalidUsername,
    #[error("User not found")]
    UserNotFound,
    #[error("Cannot send friend request to yourself")]
    SelfTarget,
    #[error("Friend request already exists")]
    AlreadyRequested,
    #[error("Already friends with this user")]
    AlreadyFriends,
    #[error("This user already sent you a friend request")]
    IncomingPending,
}

pub struct SendFriendRequest<W, U>
where
    W: FriendshipUnitOfWork,
    U: ports::user_profile::UserProfileRepository,
{
    unit_of_work: Arc<W>,
    user_repository: Arc<U>,
}

impl<W, U> SendFriendRequest<W, U>
where
    W: FriendshipUnitOfWork,
    U: ports::user_profile::UserProfileRepository,
{
    pub fn new(unit_of_work: Arc<W>, user_repository: Arc<U>) -> Self {
        Self {
            unit_of_work,
            user_repository,
        }
    }

    pub async fn execute(
        &self,
        request: SendFriendRequestRequest,
        sender_id: UserId,
    ) -> Result<UserId, SendFriendRequestError> {
        let trimmed = request.target_username.trim();
        if trimmed.is_empty() {
            return Err(SendFriendRequestError::InvalidUsername);
        }
        let username = UserName::new(trimmed.to_string());

        let target_id = self
            .user_repository
            .find_user_by_name(username.as_str())
            .await?
            .ok_or(SendFriendRequestError::UserNotFound)?;

        if target_id == sender_id {
            return Err(SendFriendRequestError::SelfTarget);
        }

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut pair = match tx.load_pair(&sender_id, &target_id).await? {
            Some(mut pair) => {
                pair.resend(sender_id, now).map_err(|e| match e {
                    FriendshipError::AlreadyFriends => SendFriendRequestError::AlreadyFriends,
                    FriendshipError::AlreadyRequested => SendFriendRequestError::AlreadyRequested,
                    FriendshipError::IncomingPending => SendFriendRequestError::IncomingPending,
                    other => SendFriendRequestError::Domain(other),
                })?;
                pair
            }
            None => FriendshipPair::request(sender_id, target_id, now)?,
        };
        let events: Vec<MessagingEvent> = pair.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_pair(&pair).await?;
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(target_id)
    }
}
