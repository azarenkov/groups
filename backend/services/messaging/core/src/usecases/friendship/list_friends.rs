use std::sync::Arc;

use messaging_domain::friendship::repository::FriendshipRepository;
use domain::{shared::repository::error::RepositoryError, user::id::UserId};

use crate::dto::{
    requests::friendship::list_friends::ListFriendsRequest,
    responses::friendship::friendship::FriendshipDto,
};

#[derive(Debug, thiserror::Error)]
pub enum ListFriendsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListFriends<F>
where
    F: FriendshipRepository,
{
    friendship_repository: Arc<F>,
}

impl<F> ListFriends<F>
where
    F: FriendshipRepository,
{
    pub fn new(friendship_repository: Arc<F>) -> Self {
        Self {
            friendship_repository,
        }
    }

    pub async fn execute(
        &self,
        request: ListFriendsRequest,
        user_id: UserId,
    ) -> Result<Vec<FriendshipDto>, ListFriendsError> {
        let rows = self
            .friendship_repository
            .list_for_user(&user_id, request.status)
            .await?;
        Ok(rows.into_iter().map(FriendshipDto::from).collect())
    }
}
