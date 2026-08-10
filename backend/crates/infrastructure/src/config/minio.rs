use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MinioConfig {
    #[serde(rename = "minio_endpoint")]
    endpoint: String,
    #[serde(rename = "minio_bucket")]
    bucket: String,
    #[serde(rename = "minio_access_key")]
    access_key: String,
    #[serde(rename = "minio_secret_key")]
    secret_key: String,
    #[serde(rename = "minio_region", default = "default_region")]
    region: String,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

impl MinioConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn access_key(&self) -> &str {
        &self.access_key
    }

    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }

    pub fn region(&self) -> &str {
        &self.region
    }
}
