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
    #[error("Missing permission to manage channels")]
    NotAllowed,
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

pub struct CreateChannel<T, H, S, R>
where
    T: ChannelRepository,
    H: Hasher,
    S: ServerRepository,
    R: domain::server::role::ServerRoleRepository,
{
    role_repository: Arc<R>,
    channel_repository: Arc<T>,
    hasher: Arc<H>,
    server_repository: Arc<S>,
}

impl<T, H, S, R> CreateChannel<T, H, S, R>
where
    T: ChannelRepository,
    H: Hasher,
    S: ServerRepository,
    R: domain::server::role::ServerRoleRepository,
{
    pub fn new(
        channel_repository: Arc<T>,
        hasher: Arc<H>,
        server_repository: Arc<S>,
        role_repository: Arc<R>,
    ) -> Self {
        Self {
            role_repository,
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

        let server_owner = self
            .server_repository
            .find_server_owner(&server_id)
            .await?;
        let allowed = server_owner.as_ref() == Some(&owner_id)
            || domain::server::role::has_permission(
                self.role_repository
                    .user_permissions(&server_id, &owner_id)
                    .await?,
                domain::server::role::PERM_MANAGE_CHANNELS,
            );
        if !allowed {
            return Err(CreateChannelError::NotAllowed);
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

        let member_ids = self
            .server_repository
            .find_member_ids_by_server_id(&server_id)
            .await?;

        let request_id = request.ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        crate::usecases::channel::recipients::inject_recipient_ids(&mut records, &member_ids);

        self.channel_repository
            .save_channel_with_events(&channel, &records)
            .await?;

        Ok(channel)
    }
}
