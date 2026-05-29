use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleOAuthConfig {
    #[serde(rename = "google_client_id", default)]
    pub client_id: String,
}

impl GoogleOAuthConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<GoogleOAuthConfig>()
    }
}
