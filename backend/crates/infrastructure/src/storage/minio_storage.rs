use application::ports::file_storage::{FileStorage, FileStorageError};
use async_trait::async_trait;
use s3::{Bucket, Region, creds::Credentials};

use crate::config::minio::MinioConfig;

pub struct MinioStorage {
    bucket: Box<Bucket>,
}

impl MinioStorage {
    pub fn new(config: &MinioConfig) -> Result<Self, FileStorageError> {
        let region = Region::Custom {
            region: config.region().to_string(),
            endpoint: config.endpoint().to_string(),
        };
        let credentials = Credentials::new(
            Some(config.access_key()),
            Some(config.secret_key()),
            None,
            None,
            None,
        )
        .map_err(|e| FileStorageError::Internal(e.to_string()))?;
        let bucket = Bucket::new(config.bucket(), region, credentials)
            .map_err(|e| FileStorageError::Internal(e.to_string()))?
            .with_path_style();
        Ok(Self { bucket })
    }

    pub async fn ensure_bucket(&self, config: &MinioConfig) -> Result<(), FileStorageError> {
        if self
            .bucket
            .exists()
            .await
            .map_err(|e| FileStorageError::Internal(e.to_string()))?
        {
            return Ok(());
        }
        let region = Region::Custom {
            region: config.region().to_string(),
            endpoint: config.endpoint().to_string(),
        };
        let credentials = Credentials::new(
            Some(config.access_key()),
            Some(config.secret_key()),
            None,
            None,
            None,
        )
        .map_err(|e| FileStorageError::Internal(e.to_string()))?;
        Bucket::create_with_path_style(
            config.bucket(),
            region,
            credentials,
            s3::BucketConfiguration::default(),
        )
        .await
        .map_err(|e| FileStorageError::Internal(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl FileStorage for MinioStorage {
    async fn put(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), FileStorageError> {
        self.bucket
            .put_object_with_content_type(key, &bytes, content_type)
            .await
            .map_err(|e| FileStorageError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, FileStorageError> {
        let response = self
            .bucket
            .get_object(key)
            .await
            .map_err(|e| FileStorageError::Internal(e.to_string()))?;
        if response.status_code() == 404 {
            return Err(FileStorageError::NotFound);
        }
        Ok(response.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<(), FileStorageError> {
        self.bucket
            .delete_object(key)
            .await
            .map_err(|e| FileStorageError::Internal(e.to_string()))?;
        Ok(())
    }
}
