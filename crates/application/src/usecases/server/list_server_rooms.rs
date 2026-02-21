use std::sync::Arc;

use domain::{
    room::Room,
    server::{repository::ServerRepository, value_objects::id::ServerId},
    shared::repository::error::RepositoryError,
};

#[derive(Debug, thiserror::Error)]
pub enum ListServerRoomsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListServerRooms<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> ListServerRooms<T>
where
    T: ServerRepository,
{
    pub fn new(server_repository: Arc<T>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(&self, server_id: ServerId) -> Result<Vec<Room>, ListServerRoomsError> {
        let rooms = self
            .server_repository
            .get_rooms_by_server_id(&server_id)
            .await?;

        Ok(rooms)
    }
}
