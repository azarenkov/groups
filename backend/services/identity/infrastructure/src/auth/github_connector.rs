use async_trait::async_trait;
use identity::ports::connections::{ConnectionProviderError, ExternalAccount, GithubConnector};
use serde::Deserialize;

use crate::auth::urlencode;

pub struct GithubOAuthConnector {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl GithubOAuthConnector {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error_description: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubUser {
    id: i64,
    login: String,
}

#[async_trait]
impl GithubConnector for GithubOAuthConnector {
    fn authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> Result<String, ConnectionProviderError> {
        if !self.is_configured() {
            return Err(ConnectionProviderError::NotConfigured);
        }
        Ok(format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&scope=read%3Auser&allow_signup=false",
            urlencode(&self.client_id),
            urlencode(redirect_uri),
            urlencode(state)
        ))
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<ExternalAccount, ConnectionProviderError> {
        if !self.is_configured() {
            return Err(ConnectionProviderError::NotConfigured);
        }
        let token: TokenResponse = self
            .http
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", redirect_uri),
            ])
            .send()
            .await
            .map_err(|e| ConnectionProviderError::Unavailable(e.to_string()))?
            .json()
            .await
            .map_err(|e| ConnectionProviderError::Unavailable(e.to_string()))?;
        let access_token = token.access_token.ok_or_else(|| {
            ConnectionProviderError::Rejected(
                token
                    .error_description
                    .or(token.error)
                    .unwrap_or_else(|| "no access token returned".to_string()),
            )
        })?;
        let user: GithubUser = self
            .http
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "rift-identity")
            .send()
            .await
            .map_err(|e| ConnectionProviderError::Unavailable(e.to_string()))?
            .error_for_status()
            .map_err(|e| ConnectionProviderError::Rejected(e.to_string()))?
            .json()
            .await
            .map_err(|e| ConnectionProviderError::Unavailable(e.to_string()))?;
        Ok(ExternalAccount {
            subject: user.id.to_string(),
            display_name: user.login,
        })
    }
}
