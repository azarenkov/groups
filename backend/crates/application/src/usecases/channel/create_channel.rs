use std::sync::Arc;

use domain::{
    channel::{
        Channel,
        repository::ChannelRepository,
        name::ChannelName, channel_type::ChannelType,
    },
    server::{repository::ServerRepository, id::ServerId},
    shared::{repository::error::RepositoryError, value_objects::password_hash::PasswordHash},
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
        let channel_type: ChannelType = request.channel_type.into();
        let password_hash = if channel_type.eq(&ChannelType::Private) {
            let password = request.password.ok_or(CreateChannelError::PasswordRequired)?;
            let hash = self.hasher.hash(&password)?;

            Some(PasswordHash::new(hash))
        } else {
            None
        };

        let channel = Channel::create(server_id, owner_id, channel_name, channel_type, password_hash);
        self.channel_repository.save_channel(&channel).await?;
        Ok(channel)
    }
}
