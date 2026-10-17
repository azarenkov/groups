use std::time::Duration;

use contracts::internal::friendships::BlockRelationResponseV1;
use domain::user::id::UserId;
use reqwest::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;

use crate::middleware::{internal::with_internal_token, request_id};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalService {
    Identity,
    Media,
    Messaging,
    Voice,
}

impl InternalService {
    fn env_var(self) -> &'static str {
        match self {
            Self::Identity => "IDENTITY_INTERNAL_URL",
            Self::Media => "MEDIA_INTERNAL_URL",
            Self::Messaging => "MESSAGING_INTERNAL_URL",
            Self::Voice => "VOICE_INTERNAL_URL",
        }
    }

    fn default_url(self) -> &'static str {
        match self {
            Self::Identity => "http://identity-service:8080",
            Self::Media => "http://media-service:8080",
            Self::Messaging => "http://messaging-service:8080",
            Self::Voice => "http://voice-service:8080",
        }
    }

    pub fn base_url(self) -> String {
        std::env::var(self.env_var()).unwrap_or_else(|_| self.default_url().to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InternalCallError {
    #[error("{0}")]
    Transport(String),
    #[error("{endpoint} endpoint returned status {status}")]
    Status { endpoint: &'static str, status: u16 },
}

impl InternalCallError {
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Status { status, .. } => Some(*status),
            Self::Transport(_) => None,
        }
    }
}

#[derive(Clone)]
pub struct InternalClient {
    client: Client,
    base_url: String,
}

pub fn with_request_id(builder: RequestBuilder) -> RequestBuilder {
    let builder = with_internal_token(builder);
    match request_id::current() {
        Some(id) => builder.header(request_id::REQUEST_ID_HEADER, id),
        None => builder,
    }
}

impl InternalClient {
    pub fn new(base_url: &str) -> Self {
        Self::with_timeout(base_url, DEFAULT_TIMEOUT)
    }

    pub fn with_timeout(base_url: &str, timeout: Duration) -> Self {
        Self {
            client: Client::builder().timeout(timeout).build().unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        with_request_id(self.client.get(format!("{}{}", self.base_url, path)))
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        with_request_id(self.client.post(format!("{}{}", self.base_url, path)))
    }
}

pub async fn send(request: RequestBuilder) -> Result<Response, InternalCallError> {
    request.send().await.map_err(|e| InternalCallError::Transport(e.to_string()))
}

pub async fn expect_success(request: RequestBuilder, endpoint: &'static str) -> Result<Response, InternalCallError> {
    let response = send(request).await?;
    if !response.status().is_success() {
        return Err(InternalCallError::Status { endpoint, status: response.status().as_u16() });
    }
    Ok(response)
}

pub async fn expect_json<T: DeserializeOwned>(request: RequestBuilder, endpoint: &'static str) -> Result<T, InternalCallError> {
    read_json(expect_success(request, endpoint).await?).await
}

pub async fn read_json<T: DeserializeOwned>(response: Response) -> Result<T, InternalCallError> {
    response.json().await.map_err(|e| InternalCallError::Transport(e.to_string()))
}

pub async fn block_relation(
    messaging: &InternalClient,
    viewer: &UserId,
    other: &UserId,
) -> Result<BlockRelationResponseV1, InternalCallError> {
    let request = messaging.get(&format!("/internal/blocks/{}/{}", viewer.uuid(), other.uuid()));
    expect_json(request, "block lookup").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_base_url_loses_its_trailing_slash_and_paths_are_appended_as_is() {
        let client = InternalClient::new("http://messaging-service:8080/");
        assert_eq!(client.base_url, "http://messaging-service:8080");
        let request = client.get("/internal/blocks/a/b").build().unwrap();
        assert_eq!(request.url().as_str(), "http://messaging-service:8080/internal/blocks/a/b");
    }

    #[test]
    fn a_failed_status_names_the_endpoint() {
        let error = InternalCallError::Status { endpoint: "block lookup", status: 503 };
        assert_eq!(error.to_string(), "block lookup endpoint returned status 503");
        assert_eq!(error.status(), Some(503));
    }
}
