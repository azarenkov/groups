use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::Channel, channel::ChannelError, channel::kind::ChannelKind, channel::name::ChannelName, channel::visibility::ChannelVisibility, community::id::CommunityId, community::policy::PolicyError, community::repository::CommunityRepository, community::unit_of_work::CommunityUnitOfWork};
use domain::{password_hash::PasswordHash, user::id::UserId};
use ports::repository::RepositoryError;

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
}

impl From<PolicyError> for CreateChannelError {
    fn from(_: PolicyError) -> Self {
        CreateChannelError::NotAllowed
    }
}

pub struct CreateChannel<H, S, W>
where
    H: Hasher,
    S: CommunityRepository,
    W: CommunityUnitOfWork,
{
    hasher: Arc<H>,
    community_repository: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<H, S, W> CreateChannel<H, S, W>
where
    H: Hasher,
    S: CommunityRepository,
    W: CommunityUnitOfWork,
{
    pub fn new(hasher: Arc<H>, community_repository: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self {
            hasher,
            community_repository,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        request: CreateChannelRequest,
        owner_id: UserId,
    ) -> Result<Channel, CreateChannelError> {
        let community_id = CommunityId::new(request.community_id);

        if !self.community_repository.community_exists(&community_id).await? {
            return Err(CreateChannelError::InvalidData);
        }

        {
            let mut tx = self.unit_of_work.begin(&community_id).await?;
            let community = tx.load_community().await?;
            community.ensure_can_manage_channels(&owner_id)?;
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
            community_id,
            owner_id,
            channel_name,
            kind,
            visibility,
            password_hash,
            now,
        )?;

        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = tx.load_community().await?;
        community.ensure_can_manage_channels(&owner_id)?;
        let member_ids = community.member_ids();

        let request_id = request.ctx.request_id.clone();
        let events = channel.pull_events();
        let events: Vec<MessagingEvent> = events.into_iter().map(|e| MessagingEvent::from(e.with_recipients(&member_ids))).collect();

        tx.insert_channel(&channel).await?;
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;

        Ok(channel)
    }
}
