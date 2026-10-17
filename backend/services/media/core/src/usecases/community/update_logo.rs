use std::sync::Arc;

use media_domain::{attachment::repository::AttachmentRepository, media::repository::MediaRepository};
use domain::{community::id::CommunityId, user::id::UserId};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        file_storage::{FileStorage, FileStorageError},
        image_moderation::{ImageModeration, ImageModerationError},
        community_owner_lookup::{CommunityOwnerLookup, CommunityOwnerLookupError},
    },
    usecases::user::update_avatar::cleanup_object,
};

#[derive(Debug, thiserror::Error)]
pub enum UpdateCommunityLogoError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Moderation error: {0}")]
    Moderation(#[from] ImageModerationError),
    #[error("Community not found")]
    CommunityNotFound,
    #[error("Not the community owner")]
    NotOwner,
    #[error("Owner lookup unavailable: {0}")]
    OwnerLookup(#[from] CommunityOwnerLookupError),
}

pub struct UpdateCommunityLogo<M, A, FS, IM, O>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
    O: CommunityOwnerLookup,
{
    media_repository: Arc<M>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
    moderation: Arc<IM>,
    owner_lookup: Arc<O>,
}

impl<M, A, FS, IM, O> UpdateCommunityLogo<M, A, FS, IM, O>
where
    M: MediaRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
    O: CommunityOwnerLookup,
{
    pub fn new(
        media_repository: Arc<M>,
        attachment_repository: Arc<A>,
        storage: Arc<FS>,
        moderation: Arc<IM>,
        owner_lookup: Arc<O>,
    ) -> Self {
        Self {
            media_repository,
            attachment_repository,
            storage,
            moderation,
            owner_lookup,
        }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        requester_id: UserId,
        bytes: Option<Vec<u8>>,
    ) -> Result<(), UpdateCommunityLogoError> {
        self.ensure_community_known(&community_id).await?;
        if !self.owner_lookup.can_manage(&community_id, &requester_id).await? {
            return Err(UpdateCommunityLogoError::NotOwner);
        }

        let new_key = match bytes {
            Some(bytes) => {
                self.moderation.ensure_allowed(&bytes).await?;
                let key = format!("logos/{}/{}.png", community_id.uuid(), uuid::Uuid::new_v4());
                self.storage.put(&key, "image/png", bytes).await?;
                Some(key)
            }
            None => None,
        };

        let still_allowed = self
            .owner_lookup
            .can_manage(&community_id, &requester_id)
            .await
            .unwrap_or_default();
        if !still_allowed {
            self.discard(new_key).await;
            return Err(UpdateCommunityLogoError::NotOwner);
        }

        let old = match self
            .media_repository
            .set_community_logo_key(&community_id, new_key.as_deref())
            .await
        {
            Ok(old) => old,
            Err(e) => {
                self.discard(new_key).await;
                return Err(match e {
                    RepositoryError::NotFound => UpdateCommunityLogoError::CommunityNotFound,
                    e => e.into(),
                });
            }
        };

        if let Some(old_key) = old {
            cleanup_object(&*self.storage, &*self.attachment_repository, &old_key).await;
        }
        Ok(())
    }

    async fn ensure_community_known(&self, community_id: &CommunityId) -> Result<(), UpdateCommunityLogoError> {
        if self.media_repository.get_community_owner(community_id).await?.is_some() {
            return Ok(());
        }
        let owner = self
            .owner_lookup
            .find_owner(community_id)
            .await?
            .ok_or(UpdateCommunityLogoError::CommunityNotFound)?;
        if let Err(e) = self
            .media_repository
            .upsert_community_owner(community_id, &owner.user_id, owner.as_of)
            .await
        {
            tracing::warn!(error = %e, "Failed to backfill community owner projection");
        }
        Ok(())
    }

    async fn discard(&self, key: Option<String>) {
        let Some(key) = key else {
            return;
        };
        if let Err(e) = self.attachment_repository.enqueue_cleanup(std::slice::from_ref(&key)).await {
            tracing::warn!(error = %e, %key, "Failed to enqueue orphaned logo object");
        }
        cleanup_object(&*self.storage, &*self.attachment_repository, &key).await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use media_domain::attachment::Attachment;
    use uuid::Uuid;

    use super::*;
    use crate::ports::{community_owner_lookup::CommunityOwner, file_storage::ByteStream};

    #[derive(Default)]
    struct Media {
        owner: Mutex<Option<(UserId, DateTime<Utc>)>>,
        logo: Mutex<Option<String>>,
    }

    #[async_trait]
    impl MediaRepository for Media {
        async fn get_user_avatar_key(&self, _: &UserId) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn set_user_avatar_key(&self, _: &UserId, _: Option<&str>) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn mark_user_deleted(&self, _: &UserId, _: DateTime<Utc>) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn get_community_logo_key(&self, _: &CommunityId) -> Result<Option<String>, RepositoryError> {
            Ok(self.logo.lock().unwrap().clone())
        }
        async fn set_community_logo_key(&self, _: &CommunityId, key: Option<&str>) -> Result<Option<String>, RepositoryError> {
            if self.owner.lock().unwrap().is_none() {
                return Err(RepositoryError::NotFound);
            }
            Ok(self.logo.lock().unwrap().replace(key.unwrap_or_default().to_string()))
        }
        async fn get_community_owner(&self, _: &CommunityId) -> Result<Option<UserId>, RepositoryError> {
            Ok(self.owner.lock().unwrap().map(|(owner, _)| owner))
        }
        async fn upsert_community_owner(&self, _: &CommunityId, owner: &UserId, at: DateTime<Utc>) -> Result<(), RepositoryError> {
            *self.owner.lock().unwrap() = Some((*owner, at));
            Ok(())
        }
        async fn remove_community_media(&self, _: &CommunityId, _: &UserId, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct Attachments;

    #[async_trait]
    impl AttachmentRepository for Attachments {
        async fn insert_reserving_quota(&self, _: &Attachment, _: i64, _: i64) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn set_size(&self, _: Uuid, _: i64) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn find_by_id(&self, _: Uuid) -> Result<Option<Attachment>, RepositoryError> {
            unimplemented!()
        }
        async fn find_unclaimed_owned(&self, _: Uuid, _: &UserId) -> Result<Option<Attachment>, RepositoryError> {
            unimplemented!()
        }
        async fn claim_owned(&self, _: &[Uuid], _: &UserId, _: Uuid) -> Result<Vec<Attachment>, RepositoryError> {
            unimplemented!()
        }
        async fn release_claims(&self, _: &[Uuid]) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn release_claims_by_token(&self, _: &[Uuid], _: Uuid) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn list_stale_claims(&self, _: i64, _: i64, _: Option<(DateTime<Utc>, Uuid)>) -> Result<Vec<(Uuid, DateTime<Utc>)>, RepositoryError> {
            unimplemented!()
        }
        async fn delete_by_storage_keys(&self, _: &[String]) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn claim_unlinked(&self, _: Uuid, _: &UserId) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn enqueue_cleanup(&self, _: &[String]) -> Result<(), RepositoryError> {
            Ok(())
        }
        async fn list_cleanup_queue(&self, _: i64) -> Result<Vec<String>, RepositoryError> {
            unimplemented!()
        }
        async fn dequeue_cleanup(&self, _: &str) -> Result<(), RepositoryError> {
            Ok(())
        }
        async fn restore_claims(&self, _: &[Uuid]) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn list_orphans_older_than(&self, _: i64, _: i64) -> Result<Vec<Uuid>, RepositoryError> {
            unimplemented!()
        }
        async fn claim_orphan(&self, _: Uuid) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn delete_orphan_row(&self, _: Uuid) -> Result<(), RepositoryError> {
            unimplemented!()
        }
    }

    #[derive(Default)]
    struct Objects {
        keys: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl FileStorage for Objects {
        async fn put(&self, key: &str, _: &str, _: Vec<u8>) -> Result<(), FileStorageError> {
            self.keys.lock().unwrap().push(key.to_string());
            Ok(())
        }
        async fn put_stream(&self, _: &str, _: &str, _: ByteStream) -> Result<u64, FileStorageError> {
            unimplemented!()
        }
        async fn get_stream(&self, _: &str) -> Result<ByteStream, FileStorageError> {
            unimplemented!()
        }
        async fn delete(&self, key: &str) -> Result<(), FileStorageError> {
            self.keys.lock().unwrap().retain(|k| k != key);
            Ok(())
        }
    }

    struct Moderation;

    #[async_trait]
    impl ImageModeration for Moderation {
        async fn ensure_allowed(&self, _: &[u8]) -> Result<(), ImageModerationError> {
            Ok(())
        }
    }

    struct Messaging {
        owner: Option<CommunityOwner>,
        managers: Vec<UserId>,
        owner_lookups: Mutex<usize>,
    }

    impl Messaging {
        fn new(owner: Option<CommunityOwner>, managers: Vec<UserId>) -> Self {
            Self { owner, managers, owner_lookups: Mutex::new(0) }
        }
    }

    #[async_trait]
    impl CommunityOwnerLookup for Messaging {
        async fn find_owner(&self, _: &CommunityId) -> Result<Option<CommunityOwner>, CommunityOwnerLookupError> {
            *self.owner_lookups.lock().unwrap() += 1;
            Ok(self.owner)
        }
        async fn can_manage(&self, _: &CommunityId, user_id: &UserId) -> Result<bool, CommunityOwnerLookupError> {
            Ok(self.managers.contains(user_id))
        }
    }

    fn usecase(
        media: Arc<Media>,
        messaging: Arc<Messaging>,
        storage: Arc<Objects>,
    ) -> UpdateCommunityLogo<Media, Attachments, Objects, Moderation, Messaging> {
        UpdateCommunityLogo::new(media, Arc::new(Attachments), storage, Arc::new(Moderation), messaging)
    }

    #[tokio::test]
    async fn a_stale_local_owner_copy_does_not_grant_access() {
        let former_owner = UserId::generate();
        let current_owner = UserId::generate();
        let media = Arc::new(Media::default());
        *media.owner.lock().unwrap() = Some((former_owner, Utc::now()));
        let messaging = Arc::new(Messaging::new(
            Some(CommunityOwner { user_id: current_owner, as_of: Utc::now() }),
            vec![current_owner],
        ));
        let storage = Arc::new(Objects::default());
        let usecase = usecase(media.clone(), messaging, storage.clone());

        let denied = usecase.execute(CommunityId::generate(), former_owner, Some(vec![1])).await;
        assert!(matches!(denied, Err(UpdateCommunityLogoError::NotOwner)));
        assert!(storage.keys.lock().unwrap().is_empty());
        assert!(media.logo.lock().unwrap().is_none());

        usecase.execute(CommunityId::generate(), current_owner, Some(vec![1])).await.unwrap();
        assert_eq!(storage.keys.lock().unwrap().len(), 1);
        assert!(media.logo.lock().unwrap().is_some());
    }

    #[tokio::test]
    async fn a_missing_owner_copy_is_backfilled_with_the_source_time() {
        let owner = UserId::generate();
        let since = Utc::now() - chrono::Duration::days(3);
        let media = Arc::new(Media::default());
        let messaging = Arc::new(Messaging::new(Some(CommunityOwner { user_id: owner, as_of: since }), vec![owner]));
        let usecase = usecase(media.clone(), messaging.clone(), Arc::new(Objects::default()));

        usecase.execute(CommunityId::generate(), owner, None).await.unwrap();
        assert_eq!(*media.owner.lock().unwrap(), Some((owner, since)));

        usecase.execute(CommunityId::generate(), owner, None).await.unwrap();
        assert_eq!(*messaging.owner_lookups.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn an_unknown_community_is_reported_before_any_authorization() {
        let requester = UserId::generate();
        let messaging = Arc::new(Messaging::new(None, vec![requester]));
        let usecase = usecase(Arc::new(Media::default()), messaging, Arc::new(Objects::default()));

        let missing = usecase.execute(CommunityId::generate(), requester, None).await;
        assert!(matches!(missing, Err(UpdateCommunityLogoError::CommunityNotFound)));
    }
}
