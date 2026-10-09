use async_trait::async_trait;
use domain::user::id::UserId;
use identity::ports::sticker_lookup::{StickerLookup, StickerLookupError};
use uuid::Uuid;

pub struct MediaStickerClient {
    client: reqwest::Client,
    base_url: String,
}

impl MediaStickerClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl StickerLookup for MediaStickerClient {
    async fn usable_sticker(&self, user_id: &UserId, sticker_id: Uuid) -> Result<bool, StickerLookupError> {
        let response = super::with_request_id(self.client.post(format!(
            "{}/internal/users/{}/status-stickers/{}",
            self.base_url,
            user_id.uuid(),
            sticker_id
        )))
        .send()
        .await
        .map_err(|e| StickerLookupError::Unavailable(e.to_string()))?;
        match response.status().as_u16() {
            200..=299 => Ok(true),
            404 => Ok(false),
            status => Err(StickerLookupError::Unavailable(format!(
                "status sticker endpoint returned status {status}"
            ))),
        }
    }
}
