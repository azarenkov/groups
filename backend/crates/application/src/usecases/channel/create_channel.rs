use std::sync::Arc;

use chrono::Utc;
use domain::{
    channel::{
        Channel, ChannelError, kind::ChannelKind, name::ChannelName, repository::ChannelRepository,
        visibility::ChannelVisibility,
    },
    server::{id::ServerId, repository::ServerRepository},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::id::UserId,
};

use crate::{
    dto::requests::channel::create_channel::CreateChannelRequest,
    ports::hasher::{HashError, Hasher},
};

#[derive(Debug, thiserror::Error)]
pub enum CreateChannelError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Password must be set")]
    PasswordRequired,
    #[error("Invalid data")]
    InvalidData,
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
}

pub struct CreateChannel<T, H, S>
where
    T: ChannelRepository,
    H: Hasher,
    S: ServerRepository,
{
    channel_repository: Arc<T>,
    hasher: Arc<H>,
    server_repository: Arc<S>,
}

impl<T, H, S> CreateChannel<T, H, S>
where
    T: ChannelRepository,
    H: Hasher,
    S: ServerRepository,
{
    pub fn new(channel_repository: Arc<T>, hasher: Arc<H>, server_repository: Arc<S>) -> Self {
        Self {
            channel_repository,
            hasher,
            server_repository,
        }
    }

    pub async fn execute(
        &self,
        request: CreateChannelRequest,
        owner_id: UserId,
    ) -> Result<Channel, CreateChannelError> {
        let server_id = ServerId::new(request.server_id);

        if !self.server_repository.server_exists(&server_id).await? {
            return Err(CreateChannelError::InvalidData);
        }

        let channel_name = ChannelName::new(request.name);
        let kind: ChannelKind = request.kind.into();
        let visibility: ChannelVisibility = request.visibility.into();
        let password_hash = if visibility.eq(&ChannelVisibility::Private) {
            let password = request
                .password
                .ok_or(CreateChannelError::PasswordRequired)?;
            let hash = self.hasher.hash(&password)?;

            Some(PasswordHash::new(hash))
        } else {
            None
        };

        let now = Utc::now();

        let mut channel = Channel::create(
            server_id,
            owner_id,
            channel_name,
            kind,
            visibility,
            password_hash,
            now,
        )?;

        let request_id = request.ctx.request_id.clone();
        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        self.channel_repository
            .save_channel_with_events(&channel, &records)
            .await?;

        Ok(channel)
    }
}
