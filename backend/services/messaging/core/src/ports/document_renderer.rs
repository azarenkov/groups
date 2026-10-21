use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum DocumentRenderError {
    #[error("Document rendering is not configured")]
    Disabled,
    #[error("Document renderer is unavailable: {0}")]
    Unavailable(String),
    #[error("The document cannot be rendered: {0}")]
    Rejected(String),
}

#[async_trait]
pub trait DocumentRenderer {
    async fn render_pdf(&self, input_name: &str, document: Vec<u8>) -> Result<Vec<u8>, DocumentRenderError>;
}
