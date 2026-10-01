use std::sync::Arc;

use chrono::Utc;
use futures::{StreamExt, TryStreamExt};
use domain::user::id::UserId;
use media_domain::{
    attachment::repository::AttachmentRepository,
    banner::BannerRepository,
    sticker::{
        AddStickerOutcome, InstallOutcome, NewSticker, StickerPack, StickerRef, StickerRepository, max_owned_packs,
        valid_pack_name, valid_pack_slug, valid_sticker_emoji,
    },
};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::{
    file_storage::{ByteStream, FileStorage, FileStorageError},
    image_moderation::{ImageModeration, ImageModerationError},
};
use crate::usecases::user::update_avatar::cleanup_object;

pub const SEARCH_PAGE_SIZE: i64 = 20;
const MAX_SEARCH_OFFSET: i64 = 2000;
const MODERATION_CONCURRENCY: usize = 4;

#[derive(Debug, thiserror::Error)]
pub enum StickerError {
    #[error("Invalid pack name")]
    InvalidName,
    #[error("Invalid pack link")]
    InvalidSlug,
    #[error("Invalid sticker emoji")]
    InvalidEmoji,
    #[error("Pack name or link is taken")]
    Taken,
    #[error("Pack limit reached for this plan")]
    PackLimit,
    #[error("The pack is full")]
    PackFull,
    #[error("Sticker pack not found")]
    PackNotFound,
    #[error("Sticker not found")]
    StickerNotFound,
    #[error("The new order must list every sticker of the pack once")]
    InvalidOrder,
    #[error("Too many installed packs")]
    InstallLimit,
    #[error("Repository error: {0}")]
    Repository(RepositoryError),
    #[error("Storage error: {0}")]
    Storage(#[from] FileStorageError),
    #[error("Moderation error: {0}")]
    Moderation(#[from] ImageModerationError),
}

impl From<RepositoryError> for StickerError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::AlreadyExists => StickerError::Taken,
            other => StickerError::Repository(other),
        }
    }
}

pub struct StickerFile {
    pub bytes: Vec<u8>,
    pub animated: bool,
    pub width: u32,
    pub height: u32,
    pub moderation_samples: Vec<Vec<u8>>,
}

impl StickerFile {
    fn extension(&self) -> &'static str {
        if self.animated { "gif" } else { "png" }
    }

    pub fn content_type(&self) -> &'static str {
        if self.animated { "image/gif" } else { "image/png" }
    }
}

pub fn is_complete_order(pack: &StickerPack, order: &[Uuid]) -> bool {
    let mut wanted: Vec<Uuid> = order.to_vec();
    wanted.sort();
    wanted.dedup();
    let mut live: Vec<Uuid> = pack.stickers.iter().map(|s| s.id).collect();
    live.sort();
    wanted.len() == order.len() && wanted == live
}

pub fn sticker_content_type(storage_key: &str) -> &'static str {
    if storage_key.ends_with(".gif") { "image/gif" } else { "image/png" }
}

pub struct StickerPacks<S, B, A, FS, IM>
where
    S: StickerRepository,
    B: BannerRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
{
    stickers: Arc<S>,
    tiers: Arc<B>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
    moderation: Arc<IM>,
}

impl<S, B, A, FS, IM> StickerPacks<S, B, A, FS, IM>
where
    S: StickerRepository,
    B: BannerRepository,
    A: AttachmentRepository,
    FS: FileStorage,
    IM: ImageModeration,
{
    pub fn new(stickers: Arc<S>, tiers: Arc<B>, attachment_repository: Arc<A>, storage: Arc<FS>, moderation: Arc<IM>) -> Self {
        Self {
            stickers,
            tiers,
            attachment_repository,
            storage,
            moderation,
        }
    }

    pub async fn create_pack(&self, owner_id: UserId, name: &str, slug: &str) -> Result<StickerPack, StickerError> {
        let name = name.trim();
        let slug = slug.trim().to_lowercase();
        if !valid_pack_name(name) {
            return Err(StickerError::InvalidName);
        }
        if !valid_pack_slug(&slug) {
            return Err(StickerError::InvalidSlug);
        }
        let tier = self.tiers.tier_of(&owner_id, Utc::now()).await?;
        let id = Uuid::new_v4();
        if !self
            .stickers
            .create_pack(id, &owner_id, name, &slug, max_owned_packs(&tier))
            .await?
        {
            return Err(StickerError::PackLimit);
        }
        self.stickers
            .find_pack(&id.to_string())
            .await?
            .ok_or(StickerError::PackNotFound)
    }

    pub async fn rename_pack(&self, owner_id: UserId, pack_id: Uuid, name: &str) -> Result<(), StickerError> {
        let name = name.trim();
        if !valid_pack_name(name) {
            return Err(StickerError::InvalidName);
        }
        if !self.stickers.rename_pack(pack_id, &owner_id, name).await? {
            return Err(StickerError::PackNotFound);
        }
        Ok(())
    }

    pub async fn delete_pack(&self, owner_id: UserId, pack_id: Uuid) -> Result<(), StickerError> {
        if !self.stickers.delete_pack(pack_id, &owner_id).await? {
            return Err(StickerError::PackNotFound);
        }
        Ok(())
    }

    pub async fn add_sticker(
        &self,
        owner_id: UserId,
        pack_id: Uuid,
        emoji: &str,
        file: StickerFile,
    ) -> Result<StickerPack, StickerError> {
        let emoji = emoji.trim();
        if !valid_sticker_emoji(emoji) {
            return Err(StickerError::InvalidEmoji);
        }
        let pack = self
            .stickers
            .find_pack(&pack_id.to_string())
            .await?
            .filter(|pack| pack.owner_id == owner_id)
            .ok_or(StickerError::PackNotFound)?;
        if pack.sticker_count >= media_domain::sticker::MAX_STICKERS_PER_PACK {
            return Err(StickerError::PackFull);
        }
        if file.moderation_samples.is_empty() {
            self.moderation.ensure_allowed(&file.bytes).await?;
        }
        futures::stream::iter(file.moderation_samples.iter())
            .map(|sample| self.moderation.ensure_allowed(sample))
            .buffer_unordered(MODERATION_CONCURRENCY)
            .try_collect::<Vec<()>>()
            .await?;
        let id = Uuid::new_v4();
        let key = format!("stickers/{}/{}.{}", pack_id, id, file.extension());
        self.storage
            .put(&key, file.content_type(), file.bytes)
            .await?;
        let sticker = NewSticker {
            id,
            storage_key: key.clone(),
            animated: file.animated,
            width: file.width as i32,
            height: file.height as i32,
            emoji: emoji.to_string(),
        };
        let outcome = match self.stickers.add_sticker(pack_id, &owner_id, &sticker).await {
            Ok(outcome) => outcome,
            Err(e) => {
                self.discard(&key).await;
                return Err(e.into());
            }
        };
        match outcome {
            AddStickerOutcome::Added => self
                .stickers
                .find_pack(&pack_id.to_string())
                .await?
                .ok_or(StickerError::PackNotFound),
            AddStickerOutcome::PackFull => {
                self.discard(&key).await;
                Err(StickerError::PackFull)
            }
            AddStickerOutcome::PackNotFound => {
                self.discard(&key).await;
                Err(StickerError::PackNotFound)
            }
        }
    }

    async fn discard(&self, key: &str) {
        if let Err(e) = self
            .attachment_repository
            .enqueue_cleanup(&[key.to_string()])
            .await
        {
            tracing::warn!(error = %e, %key, "Failed to enqueue an unused sticker object");
        }
        cleanup_object(&*self.storage, &*self.attachment_repository, key).await;
    }

    pub async fn delete_sticker(&self, owner_id: UserId, pack_id: Uuid, sticker_id: Uuid) -> Result<(), StickerError> {
        if !self
            .stickers
            .delete_sticker(pack_id, sticker_id, &owner_id)
            .await?
        {
            return Err(StickerError::StickerNotFound);
        }
        Ok(())
    }

    pub async fn reorder(&self, owner_id: UserId, pack_id: Uuid, order: &[Uuid]) -> Result<(), StickerError> {
        let pack = self
            .stickers
            .find_pack(&pack_id.to_string())
            .await?
            .filter(|pack| pack.owner_id == owner_id)
            .ok_or(StickerError::PackNotFound)?;
        if !is_complete_order(&pack, order) {
            return Err(StickerError::InvalidOrder);
        }
        if !self.stickers.reorder_stickers(pack_id, &owner_id, order).await? {
            return Err(StickerError::PackNotFound);
        }
        Ok(())
    }

    pub async fn search(&self, query: &str, page: i64) -> Result<Vec<StickerPack>, StickerError> {
        let offset = page.clamp(0, MAX_SEARCH_OFFSET / SEARCH_PAGE_SIZE) * SEARCH_PAGE_SIZE;
        Ok(self
            .stickers
            .search_packs(query, SEARCH_PAGE_SIZE, offset)
            .await?)
    }

    pub async fn pack(&self, id_or_slug: &str) -> Result<StickerPack, StickerError> {
        self.stickers
            .find_pack(id_or_slug)
            .await?
            .ok_or(StickerError::PackNotFound)
    }

    pub async fn my_packs(&self, user_id: UserId) -> Result<Vec<StickerPack>, StickerError> {
        Ok(self.stickers.packs_of(&user_id).await?)
    }

    pub async fn install(&self, user_id: UserId, pack_id: Uuid) -> Result<(), StickerError> {
        match self.stickers.install(&user_id, pack_id).await? {
            InstallOutcome::Installed => Ok(()),
            InstallOutcome::PackNotFound => Err(StickerError::PackNotFound),
            InstallOutcome::LimitReached => Err(StickerError::InstallLimit),
        }
    }

    pub async fn uninstall(&self, user_id: UserId, pack_id: Uuid) -> Result<(), StickerError> {
        self.stickers.uninstall(&user_id, pack_id).await?;
        Ok(())
    }

    pub async fn sticker(&self, sticker_id: Uuid) -> Result<StickerRef, StickerError> {
        self.stickers
            .find_sticker(sticker_id)
            .await?
            .ok_or(StickerError::StickerNotFound)
    }

    pub async fn claim_for_message(&self, sticker_id: Uuid) -> Result<StickerRef, StickerError> {
        self.stickers
            .claim_sticker(sticker_id)
            .await?
            .ok_or(StickerError::StickerNotFound)
    }

    pub async fn sticker_file(&self, sticker_id: Uuid) -> Result<(ByteStream, &'static str), StickerError> {
        let sticker = self.sticker(sticker_id).await?.sticker;
        let stream = self
            .storage
            .get_stream(&sticker.storage_key)
            .await
            .map_err(|e| match e {
                FileStorageError::NotFound => StickerError::StickerNotFound,
                other => StickerError::Storage(other),
            })?;
        Ok((stream, sticker_content_type(&sticker.storage_key)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_domain::sticker::Sticker;

    fn pack_with(ids: &[Uuid]) -> StickerPack {
        StickerPack {
            id: Uuid::nil(),
            owner_id: UserId::new(Uuid::nil()),
            name: "Pack".to_string(),
            slug: "pack".to_string(),
            install_count: 1,
            sticker_count: ids.len() as i64,
            created_at: Utc::now(),
            stickers: ids
                .iter()
                .enumerate()
                .map(|(i, id)| Sticker {
                    id: *id,
                    pack_id: Uuid::nil(),
                    storage_key: String::new(),
                    animated: false,
                    width: 512,
                    height: 512,
                    emoji: "😺".to_string(),
                    position: i as i32,
                })
                .collect(),
        }
    }

    #[test]
    fn an_order_must_list_every_sticker_once() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let pack = pack_with(&[a, b]);
        assert!(is_complete_order(&pack, &[b, a]));
        assert!(!is_complete_order(&pack, &[b]));
        assert!(!is_complete_order(&pack, &[a, a]));
        assert!(!is_complete_order(&pack, &[a, b, a]));
        assert!(!is_complete_order(&pack, &[a, c]));
    }
}
