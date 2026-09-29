use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use media_domain::{banner::BannerRepository, media::repository::MediaRepository};
use ports::repository::RepositoryError;

use crate::ports::file_storage::{ByteStream, FileStorage, FileStorageError};
use crate::usecases::user::update_avatar::still_avatar_key;

#[derive(Debug, thiserror::Error)]
pub enum GetUserAvatarError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(FileStorageError),
    #[error("Avatar not found")]
    NotFound,
}

pub struct GetUserAvatar<M, B, FS>
where
    M: MediaRepository,
    B: BannerRepository + ?Sized,
    FS: FileStorage,
{
    media_repository: Arc<M>,
    tiers: Arc<B>,
    storage: Arc<FS>,
}

impl<M, B, FS> GetUserAvatar<M, B, FS>
where
    M: MediaRepository,
    B: BannerRepository + ?Sized,
    FS: FileStorage,
{
    pub fn new(media_repository: Arc<M>, tiers: Arc<B>, storage: Arc<FS>) -> Self {
        Self {
            media_repository,
            tiers,
            storage,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<(ByteStream, &'static str), GetUserAvatarError> {
        let key = self
            .media_repository
            .get_user_avatar_key(&user_id)
            .await?
            .ok_or(GetUserAvatarError::NotFound)?;
        if let Some(still_key) = still_avatar_key(&key)
            && !self.tiers.is_paid(&user_id, Utc::now()).await?
        {
            match self.storage.get_stream(&still_key).await {
                Ok(stream) => return Ok((stream, "image/png")),
                Err(FileStorageError::NotFound) => {}
                Err(other) => return Err(GetUserAvatarError::Storage(other)),
            }
        }
        let content_type = if key.ends_with(".gif") {
            "image/gif"
        } else {
            "image/png"
        };
        let stream = self.storage.get_stream(&key).await.map_err(|e| match e {
            FileStorageError::NotFound => GetUserAvatarError::NotFound,
            other => GetUserAvatarError::Storage(other),
        })?;
        Ok((stream, content_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::DateTime;
    use domain::community::id::CommunityId;
    use futures::StreamExt;
    use std::collections::HashSet;

    struct Avatars(Option<&'static str>);

    #[async_trait]
    impl MediaRepository for Avatars {
        async fn get_user_avatar_key(&self, _: &UserId) -> Result<Option<String>, RepositoryError> {
            Ok(self.0.map(str::to_string))
        }
        async fn set_user_avatar_key(&self, _: &UserId, _: Option<&str>) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn mark_user_deleted(&self, _: &UserId, _: DateTime<Utc>) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn get_community_logo_key(&self, _: &CommunityId) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn set_community_logo_key(&self, _: &CommunityId, _: Option<&str>) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn get_community_owner(&self, _: &CommunityId) -> Result<Option<UserId>, RepositoryError> {
            unimplemented!()
        }
        async fn upsert_community_owner(&self, _: &CommunityId, _: &UserId, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn remove_community_media(&self, _: &CommunityId, _: &UserId, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
    }

    struct Tier(bool);

    #[async_trait]
    impl BannerRepository for Tier {
        async fn get_banner_key(&self, _: &UserId) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn set_banner_key(&self, _: &UserId, _: Option<&str>) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn apply_tier(&self, _: &UserId, _: &str, _: Option<DateTime<Utc>>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn is_paid(&self, _: &UserId, _: DateTime<Utc>) -> Result<bool, RepositoryError> {
            Ok(self.0)
        }
        async fn tier_of(&self, _: &UserId, _: DateTime<Utc>) -> Result<String, RepositoryError> {
            unimplemented!()
        }
    }

    struct Objects(HashSet<&'static str>);

    #[async_trait]
    impl FileStorage for Objects {
        async fn put(&self, _: &str, _: &str, _: Vec<u8>) -> Result<(), FileStorageError> {
            unimplemented!()
        }
        async fn put_stream(&self, _: &str, _: &str, _: ByteStream) -> Result<u64, FileStorageError> {
            unimplemented!()
        }
        async fn get_stream(&self, key: &str) -> Result<ByteStream, FileStorageError> {
            if !self.0.contains(key) {
                return Err(FileStorageError::NotFound);
            }
            let body = key.as_bytes().to_vec();
            Ok(Box::pin(futures::stream::once(async move { Ok(body) })))
        }
        async fn delete(&self, _: &str) -> Result<(), FileStorageError> {
            unimplemented!()
        }
    }

    async fn serve(key: &'static str, paid: bool, objects: &[&'static str]) -> (String, &'static str) {
        let usecase = GetUserAvatar::new(
            Arc::new(Avatars(Some(key))),
            Arc::new(Tier(paid)),
            Arc::new(Objects(objects.iter().copied().collect())),
        );
        let (mut stream, content_type) = usecase.execute(UserId::new(uuid::Uuid::nil())).await.unwrap();
        let body = stream.next().await.unwrap().unwrap();
        (String::from_utf8(body).unwrap(), content_type)
    }

    #[tokio::test]
    async fn paid_accounts_get_the_animation() {
        let served = serve("a/1.gif", true, &["a/1.gif", "a/1.still.png"]).await;
        assert_eq!(served, ("a/1.gif".to_string(), "image/gif"));
    }

    #[tokio::test]
    async fn free_accounts_get_the_still_frame() {
        let served = serve("a/1.gif", false, &["a/1.gif", "a/1.still.png"]).await;
        assert_eq!(served, ("a/1.still.png".to_string(), "image/png"));
    }

    #[tokio::test]
    async fn older_animations_without_a_still_keep_playing() {
        let served = serve("a/1.gif", false, &["a/1.gif"]).await;
        assert_eq!(served, ("a/1.gif".to_string(), "image/gif"));
    }

    #[tokio::test]
    async fn static_avatars_ignore_the_plan() {
        let served = serve("a/1.png", false, &["a/1.png"]).await;
        assert_eq!(served, ("a/1.png".to_string(), "image/png"));
    }
}
