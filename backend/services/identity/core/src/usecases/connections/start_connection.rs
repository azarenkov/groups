use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::connections::{ConnectionProviderError, GithubConnector, SteamConnector};

#[derive(Debug, thiserror::Error)]
pub enum StartConnectionError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Unknown provider")]
    UnknownProvider,
    #[error("Provider error: {0}")]
    Provider(#[from] ConnectionProviderError),
}

pub struct StartConnection<U, G, S>
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

impl<U, G, S> StartConnection<U, G, S>
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
        user_id: UserId,
        provider: &str,
    ) -> Result<String, StartConnectionError> {
        let state = Uuid::new_v4();
        let url = match provider {
            "github" => {
                let redirect_uri =
                    format!("{}/api/v1/connections/github/callback", self.api_base_url);
                self.github.authorize_url(&redirect_uri, &state.to_string())?
            }
            "steam" => {
                let return_to = format!(
                    "{}/api/v1/connections/steam/callback?state={}",
                    self.api_base_url, state
                );
                self.steam.authorize_url(&self.api_base_url, &return_to)
            }
            _ => return Err(StartConnectionError::UnknownProvider),
        };
        self.user_repository
            .create_connection_state(state, &user_id, provider)
            .await?;
        Ok(url)
    }
}
