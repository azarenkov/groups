use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::{
    blocks::{RelationDirectory, RelationLookupError},
    directory::RecipientDirectory,
};

pub struct BlockAwareDirectory<D: RecipientDirectory, B: RelationDirectory> {
    inner: Arc<D>,
    blocks: Arc<B>,
}

impl<D: RecipientDirectory, B: RelationDirectory> BlockAwareDirectory<D, B> {
    pub fn new(inner: Arc<D>, blocks: Arc<B>) -> Self {
        Self { inner, blocks }
    }
}

fn lookup_failed(e: RelationLookupError) -> RepositoryError {
    RepositoryError::ConnectionError(e.to_string())
}

#[async_trait]
impl<D: RecipientDirectory, B: RelationDirectory> RecipientDirectory for BlockAwareDirectory<D, B> {
    async fn community_members(&self, community_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        self.inner.community_members(community_id).await
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

    async fn pending_dm_peers(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        self.inner.pending_dm_peers(user_id).await
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
