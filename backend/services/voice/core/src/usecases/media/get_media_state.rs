use std::sync::Arc;

use domain::user::id::UserId;
use ports::repository::RepositoryError;
use voice_domain::media::{MediaState, repository::MediaStateRepository};

pub struct GetMediaState<R>
where
    R: MediaStateRepository,
{
    repository: Arc<R>,
}

impl<R> GetMediaState<R>
where
    R: MediaStateRepository,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<MediaState, RepositoryError> {
        Ok(self.repository.find(&user_id).await?.unwrap_or_default())
    }
}
