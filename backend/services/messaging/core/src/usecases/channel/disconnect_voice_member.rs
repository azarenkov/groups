use std::sync::Arc;

use messaging_domain::{channel::ChannelError, channel::id::ChannelId, channel::repository::ChannelRepository, community::id::CommunityId, community::policy::PolicyError, community::unit_of_work::CommunityUnitOfWork};
use domain::user::id::UserId;
use ports::{repository::RepositoryError, voice_service::{VoiceService, VoiceServiceError}};

#[derive(Debug, thiserror::Error)]
pub enum DisconnectVoiceMemberError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Target user not found")]
    TargetNotMember,
    #[error("Voice not supported for this channel")]
    VoiceNotSupported,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
}

impl From<PolicyError> for DisconnectVoiceMemberError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::TargetNotMember => DisconnectVoiceMemberError::TargetNotMember,
            _ => DisconnectVoiceMemberError::Domain(ChannelError::NotAllowed),
        }
    }
}

pub struct DisconnectVoiceMember<C, W, V>
where
    C: ChannelRepository,
    W: CommunityUnitOfWork,
    V: VoiceService,
{
    channel_repository: Arc<C>,
    unit_of_work: Arc<W>,
    voice_service: Arc<V>,
}

impl<C, W, V> DisconnectVoiceMember<C, W, V>
where
    C: ChannelRepository,
    W: CommunityUnitOfWork,
    V: VoiceService,
{
    pub fn new(channel_repository: Arc<C>, unit_of_work: Arc<W>, voice_service: Arc<V>) -> Self {
        Self {
            channel_repository,
            unit_of_work,
            voice_service,
        }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        channel_id: ChannelId,
        target: UserId,
        initiator: UserId,
    ) -> Result<(), DisconnectVoiceMemberError> {
        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await
            .map_err(|e| match e {
                RepositoryError::NotFound => DisconnectVoiceMemberError::ChannelNotFound,
                other => DisconnectVoiceMemberError::Repository(other),
            })?;
        if channel.community_id() != &community_id {
            return Err(DisconnectVoiceMemberError::ChannelNotFound);
        }
        if !channel.supports_voice() {
            return Err(DisconnectVoiceMemberError::VoiceNotSupported);
        }
        {
            let mut tx = self.unit_of_work.begin(&community_id).await?;
            let community = tx.load_community().await?;
            community.ensure_can_disconnect(&initiator, &target)?;
        }

        self.voice_service
            .remove_participant(&channel_id, &target)
            .await?;
        Ok(())
    }
}
