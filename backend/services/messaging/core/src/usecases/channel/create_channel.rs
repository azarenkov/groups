use std::sync::Arc;

use chrono::Utc;
use domain::{
    channel::{
        Channel, ChannelError, kind::ChannelKind, name::ChannelName,
        visibility::ChannelVisibility,
    },
    server::{
        id::ServerId,
        policy::{ModerationPolicy, PolicyError},
        repository::ServerRepository,
        unit_of_work::ServerUnitOfWork,
    },
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

impl From<PolicyError> for CreateChannelError {
    fn from(_: PolicyError) -> Self {
        CreateChannelError::NotAllowed
    }
}

pub struct CreateChannel<H, S, W>
where
    H: Hasher,
    S: ServerRepository,
    W: ServerUnitOfWork,
{
    hasher: Arc<H>,
    server_repository: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<H, S, W> CreateChannel<H, S, W>
where
    H: Hasher,
    S: ServerRepository,
    W: ServerUnitOfWork,
{
    pub fn new(hasher: Arc<H>, server_repository: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self {
            hasher,
            server_repository,
            unit_of_work,
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

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        ModerationPolicy::manage_channels(&snapshot, &owner_id)?;
        let member_ids: Vec<_> = snapshot.members.iter().copied().collect();

        let request_id = request.ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        crate::usecases::channel::recipients::inject_recipient_ids(&mut records, &member_ids);

        tx.insert_channel(&channel).await?;
        tx.save_outbox(&records).await?;
        tx.commit().await?;

        Ok(channel)
    }
}
