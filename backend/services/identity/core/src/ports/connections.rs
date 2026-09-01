use std::collections::HashMap;

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ExternalAccount {
    pub subject: String,
    pub display_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionProviderError {
    #[error("Provider is not configured")]
    NotConfigured,
    #[error("Provider rejected the request: {0}")]
    Rejected(String),
    #[error("Provider unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait GithubConnector: Send + Sync {
    fn authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> Result<String, ConnectionProviderError>;
    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<ExternalAccount, ConnectionProviderError>;
}

#[async_trait]
pub trait SteamConnector: Send + Sync {
    fn authorize_url(&self, realm: &str, return_to: &str) -> String;
    async fn verify(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<ExternalAccount, ConnectionProviderError>;
}
