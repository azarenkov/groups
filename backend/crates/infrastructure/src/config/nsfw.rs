use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct NsfwConfig {
    #[serde(rename = "nsfw_detector_url", default)]
    pub detector_url: Option<String>,
    #[serde(rename = "nsfw_threshold", default = "default_threshold")]
    pub threshold: f64,
}

fn default_threshold() -> f64 {
    0.8
}

impl NsfwConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
