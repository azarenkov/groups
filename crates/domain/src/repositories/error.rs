#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Room not found")]
    NotFound,

    #[error("Room with this name already exists")]
    AlreadyExists,

    #[error("Database connection error: {0}")]
    ConnectionError(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
