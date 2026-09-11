use std::sync::Arc;

use messaging_domain::{friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus};
use domain::{pagination::{KeysetCursor, Page}, user::id::UserId};
use ports::repository::RepositoryError;

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
    ) -> Result<Page<FriendshipDto>, ListFriendsError> {
        let rows = self
            .friendship_repository
            .list_for_user(&user_id, request.status, request.cursor.as_ref(), request.limit)
            .await?;
        let page = Page::from_overfetch(rows, request.limit, |row| {
            KeysetCursor::new(row.updated_at, row.friend_id.uuid().to_string())
        });
        Ok(page.map(FriendshipDto::from))
    }

    pub async fn statuses(
        &self,
        user_id: UserId,
    ) -> Result<Vec<(UserId, FriendshipStatus)>, ListFriendsError> {
        Ok(self
            .friendship_repository
            .list_statuses_for_user(&user_id)
            .await?)
    }
}
