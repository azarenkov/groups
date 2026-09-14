use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;

use crate::ports::block_lookup::{BlockLookup, BlockLookupError};

use super::list_connections::{ConnectionView, ListConnections, ListConnectionsError};

#[derive(Debug, thiserror::Error)]
pub enum ListUserConnectionsError {
    #[error(transparent)]
    Connections(#[from] ListConnectionsError),
    #[error("Block lookup unavailable: {0}")]
    BlockLookup(#[from] BlockLookupError),
}

pub struct ListUserConnections<U: UserRepository, B: BlockLookup> {
    connections: ListConnections<U>,
    block_lookup: Arc<B>,
}

impl<U: UserRepository, B: BlockLookup> ListUserConnections<U, B> {
    pub fn new(user_repository: Arc<U>, block_lookup: Arc<B>) -> Self {
        Self {
            connections: ListConnections::new(user_repository),
            block_lookup,
        }
    }

    pub async fn execute(
        &self,
        viewer_id: UserId,
        target_id: UserId,
    ) -> Result<Vec<ConnectionView>, ListUserConnectionsError> {
        if viewer_id == target_id {
            return Ok(self.connections.execute(target_id).await?);
        }
        if self.block_lookup.blocked_by(&viewer_id, &target_id).await? {
            return Ok(Vec::new());
        }
        let connections = self.connections.execute(target_id).await?;
        if self.block_lookup.blocked_by(&viewer_id, &target_id).await? {
            return Ok(Vec::new());
        }
        Ok(connections)
    }
}
