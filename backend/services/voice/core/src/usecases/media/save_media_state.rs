use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use voice_domain::media::{MediaState, repository::MediaStateRepository};

pub struct SaveMediaState<R>
where
    R: MediaStateRepository,
{
    repository: Arc<R>,
}

impl<R> SaveMediaState<R>
where
    R: MediaStateRepository,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        mic_muted: bool,
        deafened: bool,
        now: DateTime<Utc>,
    ) -> Result<MediaState, RepositoryError> {
        let state = MediaState::new(mic_muted, deafened);
        self.repository.save(&user_id, state, now).await?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use uuid::Uuid;

    use super::*;
    use crate::usecases::media::get_media_state::GetMediaState;

    #[derive(Default)]
    struct Store(Mutex<HashMap<Uuid, MediaState>>);

    #[async_trait]
    impl MediaStateRepository for Store {
        async fn find(&self, user_id: &UserId) -> Result<Option<MediaState>, RepositoryError> {
            Ok(self.0.lock().unwrap().get(&user_id.uuid()).copied())
        }

        async fn save(&self, user_id: &UserId, state: MediaState, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            self.0.lock().unwrap().insert(user_id.uuid(), state);
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_saved_state_is_what_the_next_session_reads_back() {
        let store = Arc::new(Store::default());
        let user = UserId::new(Uuid::new_v4());
        let read = GetMediaState::new(store.clone());
        assert_eq!(read.execute(user).await.unwrap(), MediaState::default());

        let saved = SaveMediaState::new(store.clone()).execute(user, false, true, Utc::now()).await.unwrap();
        assert!(saved.mic_muted());
        assert_eq!(read.execute(user).await.unwrap(), MediaState::new(true, true));
    }
}
