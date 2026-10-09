use async_trait::async_trait;
use contracts::internal::stickers::StickerInfoV1;
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
    async fn usable_sticker(&self, sticker_id: Uuid) -> Result<bool, StickerLookupError> {
        let response = super::with_request_id(
            self.client
                .get(format!("{}/internal/stickers/{}", self.base_url, sticker_id)),
        )
        .send()
        .await
        .map_err(|e| StickerLookupError::Unavailable(e.to_string()))?;
        match response.status().as_u16() {
            200 => {
                let info: StickerInfoV1 = response
                    .json()
                    .await
                    .map_err(|e| StickerLookupError::Unavailable(e.to_string()))?;
                Ok(!info.deleted)
            }
            404 => Ok(false),
            status => Err(StickerLookupError::Unavailable(format!(
                "sticker lookup endpoint returned status {status}"
            ))),
        }
    }
}
