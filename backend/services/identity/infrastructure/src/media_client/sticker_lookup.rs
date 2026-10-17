use api_common::internal_client::{InternalCallError, InternalClient, send};
use async_trait::async_trait;
use domain::user::id::UserId;
use identity::ports::sticker_lookup::{StickerLookup, StickerLookupError};
use uuid::Uuid;

pub struct MediaStickerClient {
    media: InternalClient,
}

impl MediaStickerClient {
    pub fn new(base_url: &str) -> Self {
        Self { media: InternalClient::new(base_url) }
    }
}

#[async_trait]
impl StickerLookup for MediaStickerClient {
    async fn usable_sticker(&self, user_id: &UserId, sticker_id: Uuid) -> Result<bool, StickerLookupError> {
        let request = self
            .media
            .post(&format!("/internal/users/{}/status-stickers/{}", user_id.uuid(), sticker_id));
        let response = send(request).await.map_err(|e| StickerLookupError::Unavailable(e.to_string()))?;
        match response.status().as_u16() {
            200..=299 => Ok(true),
            404 => Ok(false),
            status => Err(StickerLookupError::Unavailable(
                InternalCallError::Status { endpoint: "status sticker", status }.to_string(),
            )),
        }
    }
}
