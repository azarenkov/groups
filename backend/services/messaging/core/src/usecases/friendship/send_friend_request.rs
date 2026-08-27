use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, friendship::Friendship, friendship::FriendshipError, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, friendship::unit_of_work::FriendshipUnitOfWork};
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

pub struct SendFriendRequest<F, W, U>
where
    F: FriendshipRepository,
    W: FriendshipUnitOfWork,
    U: ports::user_profile::UserProfileRepository,
{
    friendship_repository: Arc<F>,
    unit_of_work: Arc<W>,
    user_repository: Arc<U>,
}

impl<F, W, U> SendFriendRequest<F, W, U>
where
    F: FriendshipRepository,
    W: FriendshipUnitOfWork,
    U: ports::user_profile::UserProfileRepository,
{
    pub fn new(friendship_repository: Arc<F>, unit_of_work: Arc<W>, user_repository: Arc<U>) -> Self {
        Self {
            friendship_repository,
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

        if let Some(existing) = self
            .friendship_repository
            .find_pair(&sender_id, &target_id)
            .await?
        {
            match existing.status {
                FriendshipStatus::Accepted => {
                    return Err(SendFriendRequestError::AlreadyFriends);
                }
                FriendshipStatus::PendingOutgoing => {
                    return Err(SendFriendRequestError::AlreadyRequested);
                }
                FriendshipStatus::PendingIncoming => {
                    return Err(SendFriendRequestError::IncomingPending);
                }
                FriendshipStatus::Declined => {}
            }
        }

        let now = Utc::now();
        let (outgoing, incoming, events) =
            Friendship::build_request_pair(sender_id, target_id, now)?;

        let request_id = request.ctx.request_id.clone();
        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let mut tx = self.unit_of_work.begin().await?;
        tx.insert_request_pair(&outgoing, &incoming).await?;
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(target_id)
    }
}
