use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::connections::{ConnectionProviderError, GithubConnector, SteamConnector};

#[derive(Debug, thiserror::Error)]
pub enum CompleteConnectionError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Provider error: {0}")]
    Provider(#[from] ConnectionProviderError),
    #[error("Invalid or expired connection state")]
    InvalidState,
    #[error("This account is already linked to another user")]
    SubjectTaken,
}

pub struct CompleteConnection<U, G, S>
where
    U: UserRepository,
    G: GithubConnector,
    S: SteamConnector,
{
    user_repository: Arc<U>,
    github: Arc<G>,
    steam: Arc<S>,
    api_base_url: String,
}

impl<U, G, S> CompleteConnection<U, G, S>
where
    U: UserRepository,
    G: GithubConnector,
    S: SteamConnector,
{
    pub fn new(user_repository: Arc<U>, github: Arc<G>, steam: Arc<S>, api_base_url: String) -> Self {
        Self {
            user_repository,
            github,
            steam,
            api_base_url,
        }
    }

    pub async fn execute(
        &self,
        provider: &str,
        params: &HashMap<String, String>,
    ) -> Result<(), CompleteConnectionError> {
        let state = params
            .get("state")
            .and_then(|raw| Uuid::parse_str(raw).ok())
            .ok_or(CompleteConnectionError::InvalidState)?;
        let (user_id, stored_provider, created_at) = self
            .user_repository
            .consume_connection_state(state)
            .await?
            .ok_or(CompleteConnectionError::InvalidState)?;
        if stored_provider != provider || Utc::now() - created_at > Duration::minutes(10) {
            return Err(CompleteConnectionError::InvalidState);
        }
        let account = match provider {
            "github" => {
                let code = params
                    .get("code")
                    .ok_or(CompleteConnectionError::InvalidState)?;
                let redirect_uri =
                    format!("{}/api/v1/connections/github/callback", self.api_base_url);
                self.github.exchange_code(code, &redirect_uri).await?
            }
            "steam" => self.steam.verify(params).await?,
            _ => return Err(CompleteConnectionError::InvalidState),
        };
        if !self
            .user_repository
            .link_connection(provider, &account.subject, &user_id, &account.display_name)
            .await?
        {
            return Err(CompleteConnectionError::SubjectTaken);
        }
        Ok(())
    }
}
