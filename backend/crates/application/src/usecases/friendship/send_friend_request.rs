use std::sync::Arc;

use chrono::Utc;
use domain::{
    friendship::{
        Friendship, FriendshipError, repository::FriendshipRepository,
        status::FriendshipStatus,
    },
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::{id::UserId, name::UserName, repository::UserRepository},
};

use crate::dto::requests::friendship::send_friend_request::SendFriendRequestRequest;

#[derive(Debug, thiserror::Error)]
pub enum SendFriendRequestError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Friendship error: {0}")]
    Domain(#[from] FriendshipError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
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

pub struct SendFriendRequest<F, U>
where
    F: FriendshipRepository,
    U: UserRepository,
{
    friendship_repository: Arc<F>,
    user_repository: Arc<U>,
}

impl<F, U> SendFriendRequest<F, U>
where
    F: FriendshipRepository,
    U: UserRepository,
{
    pub fn new(friendship_repository: Arc<F>, user_repository: Arc<U>) -> Self {
        Self {
            friendship_repository,
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

        let target = self
            .user_repository
            .find_by_name(&username)
            .await?
            .ok_or(SendFriendRequestError::UserNotFound)?;
        let target_id = *target.id();

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
        let records: Vec<OutboxRecord> = events
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        self.friendship_repository
            .save_request_with_events(&outgoing, &incoming, &records)
            .await?;

        Ok(target_id)
    }
}
