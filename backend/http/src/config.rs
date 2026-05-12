use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    #[serde(default)]
    pub trust_forwarded_headers: bool,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<ServerConfig>()
    }
}
