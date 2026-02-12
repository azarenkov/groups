use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LiveKitConfig {
    api_key: String,
    api_secret: String,
    #[serde(rename = "livekit_url")]
    url: String,
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
