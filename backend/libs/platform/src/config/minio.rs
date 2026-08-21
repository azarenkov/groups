use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MinioConfig {
    #[serde(rename = "minio_endpoint", default = "default_endpoint")]
    endpoint: String,
    #[serde(rename = "minio_bucket", default = "default_bucket")]
    bucket: String,
    #[serde(rename = "minio_access_key", default)]
    access_key: Option<String>,
    #[serde(rename = "minio_secret_key", default)]
    secret_key: Option<String>,
    #[serde(rename = "minio_root_user", default)]
    root_user: Option<String>,
    #[serde(rename = "minio_root_password", default)]
    root_password: Option<String>,
    #[serde(rename = "minio_region", default = "default_region")]
    region: String,
}

fn default_endpoint() -> String {
    "http://minio:9000".to_string()
}

fn default_bucket() -> String {
    "rift-attachments".to_string()
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
        self.access_key
            .as_deref()
            .or(self.root_user.as_deref())
            .unwrap_or("rift")
    }

    pub fn secret_key(&self) -> &str {
        self.secret_key
            .as_deref()
            .or(self.root_password.as_deref())
            .unwrap_or("rift-minio-secret")
    }

    pub fn region(&self) -> &str {
        &self.region
    }
}
