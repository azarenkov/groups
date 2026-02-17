use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NatsConfig {
    #[serde(rename = "nats_url")]
    url: String,
    #[serde(rename = "nats_subject_prefix")]
    subject_prefix: String,
}

impl NatsConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv::dotenv().ok();
        envy::from_env::<NatsConfig>()
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn subject_prefix(&self) -> &str {
        &self.subject_prefix
    }
}
