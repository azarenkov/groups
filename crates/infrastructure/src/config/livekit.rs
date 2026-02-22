use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LiveKitConfig {
    #[serde(rename = "livekit_api_key")]
    api_key: String,
    #[serde(rename = "livekit_api_secret")]
    api_secret: String,
    #[serde(rename = "livekit_url")]
    url: String,
    #[serde(rename = "livekit_client_url")]
    pub client_url: String,
}

impl LiveKitConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<LiveKitConfig>()
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn api_secret(&self) -> &str {
        &self.api_secret
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}
