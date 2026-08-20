use async_trait::async_trait;
use crate::ports::text_moderation::{TextModeration, TextModerationError};
use platform::config::nsfw::NsfwConfig;

pub struct NsfwDetector {
    client: reqwest::Client,
    url: Option<String>,
    threshold: f64,
}

impl NsfwDetector {
    pub fn new(config: &NsfwConfig) -> Self {
        let url = config
            .detector_url
            .as_ref()
            .map(|u| u.trim_end_matches('/').to_string())
            .filter(|u| !u.is_empty());
        if url.is_none() {
            tracing::warn!(
                "NSFW_DETECTOR_URL is not set; text content moderation is disabled"
            );
        }
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            url,
            threshold: config.threshold,
        }
    }
}

#[async_trait]
impl TextModeration for NsfwDetector {
    async fn ensure_allowed(&self, text: &str) -> Result<(), TextModerationError> {
        let Some(url) = &self.url else {
            return Ok(());
        };
        let response = self
            .client
            .post(format!("{url}/moderate/text"))
            .json(&serde_json::json!({ "text": text, "kind": "username" }))
            .send()
            .await
            .map_err(|e| TextModerationError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(TextModerationError::Unavailable(format!(
                "detector returned status {}",
                response.status()
            )));
        }
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| TextModerationError::Unavailable(e.to_string()))?;
        let allowed = body.get("allowed").and_then(|v| v.as_bool()).ok_or_else(|| {
            TextModerationError::Unavailable("malformed detector response".to_string())
        })?;
        if !allowed {
            return Err(TextModerationError::Rejected);
        }
        Ok(())
    }
}
