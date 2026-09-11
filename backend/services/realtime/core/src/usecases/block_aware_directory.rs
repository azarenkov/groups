use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::{
    blocks::{BlockDirectory, BlockLookupError},
    directory::RecipientDirectory,
};

pub struct BlockAwareDirectory<D: RecipientDirectory, B: BlockDirectory> {
    inner: Arc<D>,
    blocks: Arc<B>,
}

impl<D: RecipientDirectory, B: BlockDirectory> BlockAwareDirectory<D, B> {
    pub fn new(inner: Arc<D>, blocks: Arc<B>) -> Self {
        Self { inner, blocks }
    }
}

fn lookup_failed(e: BlockLookupError) -> RepositoryError {
    RepositoryError::ConnectionError(e.to_string())
}

#[async_trait]
impl<D: RecipientDirectory, B: BlockDirectory> RecipientDirectory for BlockAwareDirectory<D, B> {
    async fn server_members(&self, server_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        self.inner.server_members(server_id).await
    }

    async fn user_name(&self, user_id: Uuid) -> Result<Option<String>, RepositoryError> {
        self.inner.user_name(user_id).await
    }

    async fn user_name_versioned(&self, user_id: Uuid) -> Result<Option<(String, DateTime<Utc>)>, RepositoryError> {
        self.inner.user_name_versioned(user_id).await
    }

    async fn rename_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        self.inner.rename_recipients(user_id).await
    }

    async fn profile_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        let mut recipients = self.inner.profile_recipients(user_id).await?;
        let relations = self
            .blocks
            .block_relations(user_id, &recipients)
            .await
            .map_err(lookup_failed)?;
        recipients.retain(|r| !relations.blocking.contains(r));
        Ok(recipients)
    }

    async fn interested_in(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        let mut recipients = self.inner.interested_in(user_id).await?;
        let relations = self
            .blocks
            .block_relations(user_id, &recipients)
            .await
            .map_err(lookup_failed)?;
        recipients.retain(|r| !relations.blocking.contains(r) && !relations.blocked_by.contains(r));
        Ok(recipients)
    }
}
