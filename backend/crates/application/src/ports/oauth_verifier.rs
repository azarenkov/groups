use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProvider {
    Google,
}

impl OAuthProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "google",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OAuthIdentity {
    pub provider: OAuthProvider,
    pub subject: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthVerifyError {
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(&'static str),
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Verifier transport error: {0}")]
    Transport(String),
}

#[async_trait]
pub trait OAuthIdentityVerifier: Send + Sync {
    async fn verify(
        &self,
        provider: OAuthProvider,
        token: &str,
    ) -> Result<OAuthIdentity, OAuthVerifyError>;
}
