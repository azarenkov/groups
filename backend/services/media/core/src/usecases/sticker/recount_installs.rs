use std::sync::Arc;

use media_domain::sticker::StickerRepository;
use ports::repository::RepositoryError;

pub struct RecountStickerInstalls<R: StickerRepository> {
    repository: Arc<R>,
}

impl<R: StickerRepository> RecountStickerInstalls<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self) -> Result<u64, RepositoryError> {
        self.repository.recount_installs().await
    }
}
