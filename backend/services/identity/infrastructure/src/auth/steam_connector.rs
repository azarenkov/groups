use std::collections::HashMap;

use async_trait::async_trait;
use identity::ports::connections::{ConnectionProviderError, ExternalAccount, SteamConnector};
use serde::Deserialize;

use crate::auth::urlencode;

const STEAM_OPENID_URL: &str = "https://steamcommunity.com/openid/login";
const OPENID_NS: &str = "http://specs.openid.net/auth/2.0";
const OPENID_IDENTIFIER: &str = "http://specs.openid.net/auth/2.0/identifier_select";

pub struct SteamOpenIdConnector {
    api_key: String,
    http: reqwest::Client,
}

impl SteamOpenIdConnector {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    async fn persona_name(&self, steam_id: &str) -> Option<String> {
        if self.api_key.is_empty() {
            return None;
        }
        let response: SummaryResponse = self
            .http
            .get("https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/")
            .query(&[("key", self.api_key.as_str()), ("steamids", steam_id)])
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        response
            .response
            .players
            .into_iter()
            .next()
            .and_then(|p| p.personaname)
    }
}

#[derive(Debug, Deserialize)]
struct SummaryResponse {
    response: SummaryPlayers,
}

#[derive(Debug, Deserialize)]
struct SummaryPlayers {
    #[serde(default)]
    players: Vec<SummaryPlayer>,
}

#[derive(Debug, Deserialize)]
struct SummaryPlayer {
    personaname: Option<String>,
}

#[async_trait]
impl SteamConnector for SteamOpenIdConnector {
    fn authorize_url(&self, realm: &str, return_to: &str) -> String {
        format!(
            "{STEAM_OPENID_URL}?openid.ns={}&openid.mode=checkid_setup&openid.return_to={}&openid.realm={}&openid.identity={}&openid.claimed_id={}",
            urlencode(OPENID_NS),
            urlencode(return_to),
            urlencode(realm),
            urlencode(OPENID_IDENTIFIER),
            urlencode(OPENID_IDENTIFIER)
        )
    }

    async fn verify(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<ExternalAccount, ConnectionProviderError> {
        if params.get("openid.mode").map(String::as_str) != Some("id_res") {
            return Err(ConnectionProviderError::Rejected(
                "unexpected openid mode".to_string(),
            ));
        }
        let mut form: Vec<(String, String)> = params
            .iter()
            .filter(|(key, _)| key.starts_with("openid."))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        for (key, value) in form.iter_mut() {
            if key == "openid.mode" {
                *value = "check_authentication".to_string();
            }
        }
        let body = self
            .http
            .post(STEAM_OPENID_URL)
            .form(&form)
            .send()
            .await
            .map_err(|e| ConnectionProviderError::Unavailable(e.to_string()))?
            .text()
            .await
            .map_err(|e| ConnectionProviderError::Unavailable(e.to_string()))?;
        if !body.lines().any(|line| line.trim() == "is_valid:true") {
            return Err(ConnectionProviderError::Rejected(
                "openid verification failed".to_string(),
            ));
        }
        let steam_id = params
            .get("openid.claimed_id")
            .and_then(|claimed| claimed.rsplit('/').next())
            .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
            .ok_or_else(|| ConnectionProviderError::Rejected("invalid claimed id".to_string()))?
            .to_string();
        let display_name = self
            .persona_name(&steam_id)
            .await
            .unwrap_or_else(|| steam_id.clone());
        Ok(ExternalAccount {
            subject: steam_id,
            display_name,
        })
    }
}
