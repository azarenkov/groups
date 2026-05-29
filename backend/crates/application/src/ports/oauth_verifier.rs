use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct GoogleIdentity {
    pub subject: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthVerifyError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Verifier transport error: {0}")]
    Transport(String),
}

#[async_trait]
pub trait GoogleTokenVerifier: Send + Sync {
    async fn verify(&self, id_token: &str) -> Result<GoogleIdentity, OAuthVerifyError>;
}
