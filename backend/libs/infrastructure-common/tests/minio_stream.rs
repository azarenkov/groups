use futures::StreamExt;
use infrastructure_common::{config::minio::MinioConfig, storage::minio_storage::MinioStorage};
use ports::file_storage::{ByteStream, FileStorage, FileStorageError};

const CHUNK: usize = 1024 * 1024;

fn chunks(count: usize) -> ByteStream {
    Box::pin(futures::stream::iter(
        (0..count).map(|index| Ok::<_, FileStorageError>(vec![(index % 251) as u8; CHUNK])),
    ))
}

async fn size_of(storage: &MinioStorage, key: &str) -> usize {
    let mut stream = storage.get_stream(key).await.unwrap();
    let mut total = 0;
    while let Some(chunk) = stream.next().await {
        total += chunk.unwrap().len();
    }
    total
}

#[tokio::test]
#[ignore = "needs a MinIO reachable through the MINIO_* variables"]
async fn a_file_larger_than_one_part_arrives_whole() {
    let config = MinioConfig::from_env().unwrap();
    let storage = MinioStorage::new(&config).unwrap();
    storage.ensure_bucket(&config).await.unwrap();

    let written = storage
        .put_stream("tests/large.bin", "application/octet-stream", chunks(20))
        .await
        .unwrap();
    assert_eq!(written, (20 * CHUNK) as u64);
    assert_eq!(size_of(&storage, "tests/large.bin").await, 20 * CHUNK);

    let written = storage
        .put_stream("tests/small.bin", "application/octet-stream", chunks(1))
        .await
        .unwrap();
    assert_eq!(written, CHUNK as u64);
    assert_eq!(size_of(&storage, "tests/small.bin").await, CHUNK);

    storage.delete("tests/large.bin").await.unwrap();
    storage.delete("tests/small.bin").await.unwrap();
}

#[tokio::test]
#[ignore = "needs a MinIO reachable through the MINIO_* variables"]
async fn a_stream_that_breaks_midway_fails_the_upload() {
    let config = MinioConfig::from_env().unwrap();
    let storage = MinioStorage::new(&config).unwrap();
    storage.ensure_bucket(&config).await.unwrap();

    let broken: ByteStream = Box::pin(
        chunks(12).chain(futures::stream::iter([Err(FileStorageError::Internal(
            "too large".to_string(),
        ))])),
    );
    assert!(storage
        .put_stream("tests/broken.bin", "application/octet-stream", broken)
        .await
        .is_err());
    assert!(matches!(
        storage.get_stream("tests/broken.bin").await,
        Err(FileStorageError::NotFound)
    ));
}
