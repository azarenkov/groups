use ports::file_storage::{ByteStream, FileStorage, FileStorageError};
use async_trait::async_trait;
use futures::StreamExt;
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

fn check_status(code: u16, op: &str) -> Result<(), FileStorageError> {
    if code == 404 {
        return Err(FileStorageError::NotFound);
    }
    if !(200..300).contains(&code) {
        return Err(FileStorageError::Internal(format!(
            "{op} returned HTTP {code}"
        )));
    }
    Ok(())
}

#[async_trait]
impl FileStorage for MinioStorage {
    async fn put(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), FileStorageError> {
        let response = self
            .bucket
            .put_object_with_content_type(key, &bytes, content_type)
            .await
            .map_err(|e| FileStorageError::Internal(e.to_string()))?;
        check_status(response.status_code(), "put_object")
    }

    async fn get_stream(&self, key: &str) -> Result<ByteStream, FileStorageError> {
        let response = self
            .bucket
            .get_object_stream(key)
            .await
            .map_err(|e| match e {
                s3::error::S3Error::HttpFailWithBody(404, _) => FileStorageError::NotFound,
                other => FileStorageError::Internal(other.to_string()),
            })?;
        check_status(response.status_code, "get_object")?;
        let stream = response.bytes.map(|chunk| match chunk {
            Ok(bytes) => Ok(bytes.to_vec()),
            Err(e) => Err(FileStorageError::Internal(e.to_string())),
        });
        Ok(Box::pin(stream))
    }

    async fn delete(&self, key: &str) -> Result<(), FileStorageError> {
        let response = self
            .bucket
            .delete_object(key)
            .await
            .map_err(|e| FileStorageError::Internal(e.to_string()))?;
        check_status(response.status_code(), "delete_object")
    }
}
