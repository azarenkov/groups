use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use messaging::ports::document_renderer::{DocumentRenderError, DocumentRenderer};
use reqwest::{Client, multipart};

const CONVERT_PATH: &str = "/forms/libreoffice/convert";
const DEFAULT_TIMEOUT_SECS: u64 = 90;
const MAX_RENDITION_BYTES: usize = 100 * 1024 * 1024;

pub struct GotenbergRenderer {
    client: Client,
    endpoint: Option<String>,
}

impl GotenbergRenderer {
    pub fn new(base_url: Option<&str>, timeout: Duration) -> Self {
        let endpoint = base_url
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(|url| format!("{}{CONVERT_PATH}", url.trim_end_matches('/')));
        if endpoint.is_none() {
            tracing::warn!("DOCUMENT_CONVERTER_URL is not set; office document previews are disabled");
        }
        Self {
            client: Client::builder().timeout(timeout).build().unwrap_or_default(),
            endpoint,
        }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("DOCUMENT_CONVERTER_URL").ok();
        let timeout = std::env::var("DOCUMENT_CONVERTER_TIMEOUT_SECS")
            .ok()
            .and_then(|raw| raw.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);
        Self::new(url.as_deref(), Duration::from_secs(timeout))
    }
}

pub fn classify(status: u16) -> Option<DocumentRenderError> {
    match status {
        200..=299 => None,
        400..=499 => Some(DocumentRenderError::Rejected(format!("converter answered {status}"))),
        _ => Some(DocumentRenderError::Unavailable(format!("converter answered {status}"))),
    }
}

#[async_trait]
impl DocumentRenderer for GotenbergRenderer {
    async fn render_pdf(&self, input_name: &str, document: Vec<u8>) -> Result<Vec<u8>, DocumentRenderError> {
        let endpoint = self.endpoint.as_deref().ok_or(DocumentRenderError::Disabled)?;
        let part = multipart::Part::bytes(document).file_name(input_name.to_string());
        let form = multipart::Form::new().part("files", part);
        let response = self
            .client
            .post(endpoint)
            .multipart(form)
            .send()
            .await
            .map_err(|e| DocumentRenderError::Unavailable(e.to_string()))?;
        if let Some(error) = classify(response.status().as_u16()) {
            return Err(error);
        }
        let mut body = response.bytes_stream();
        let mut pdf = Vec::new();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| DocumentRenderError::Unavailable(e.to_string()))?;
            if pdf.len() + chunk.len() > MAX_RENDITION_BYTES {
                return Err(DocumentRenderError::Rejected("rendition exceeds the size limit".to_string()));
            }
            pdf.extend_from_slice(&chunk);
        }
        if !pdf.starts_with(b"%PDF") {
            return Err(DocumentRenderError::Rejected("converter did not return a PDF".to_string()));
        }
        Ok(pdf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_errors_reject_the_document_and_server_errors_mean_unavailable() {
        assert!(classify(200).is_none());
        assert!(matches!(classify(400), Some(DocumentRenderError::Rejected(_))));
        assert!(matches!(classify(413), Some(DocumentRenderError::Rejected(_))));
        assert!(matches!(classify(503), Some(DocumentRenderError::Unavailable(_))));
    }

    #[tokio::test]
    async fn an_unconfigured_converter_is_disabled() {
        let renderer = GotenbergRenderer::new(Some("  "), Duration::from_secs(1));
        assert!(matches!(renderer.render_pdf("document.docx", Vec::new()).await, Err(DocumentRenderError::Disabled)));
    }

    #[tokio::test]
    #[ignore = "needs a running converter; set GOTENBERG_TEST_URL"]
    async fn renders_a_document_through_a_running_converter() {
        let url = std::env::var("GOTENBERG_TEST_URL").expect("GOTENBERG_TEST_URL");
        let renderer = GotenbergRenderer::new(Some(&url), Duration::from_secs(60));
        let pdf = renderer
            .render_pdf("document.rtf", b"{\\rtf1\\ansi Groups preview}".to_vec())
            .await
            .unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn the_endpoint_is_built_from_the_base_url() {
        let renderer = GotenbergRenderer::new(Some("http://document-converter:3000/"), Duration::from_secs(1));
        assert_eq!(renderer.endpoint.as_deref(), Some("http://document-converter:3000/forms/libreoffice/convert"));
    }
}
