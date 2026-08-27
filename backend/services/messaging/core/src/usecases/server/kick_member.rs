use std::sync::Arc;

use domain::shared::outbox::to_outbox::ToOutboxRecord;

use messaging_domain::{channel::repository::ChannelRepository, server::id::ServerId, server::policy::ModerationPolicy, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};
use domain::{shared::repository::error::RepositoryError, user::id::UserId};

use crate::ports::voice_service::VoiceService;

#[derive(Debug, thiserror::Error)]
pub enum KickMemberError {
    #[error("Outbox error: {0}")]
    Outbox(#[from] domain::shared::outbox::to_outbox::OutboxError),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Target is not a member of this server")]
    NotMember,
    #[error("Missing permission to kick members")]
    NotAllowed,
}

impl From<PolicyError> for KickMemberError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::TargetNotMember => KickMemberError::NotMember,
            _ => KickMemberError::NotAllowed,
        }
    }
}

pub struct KickMember<W, C, V>
where
    W: ServerUnitOfWork,
    C: ChannelRepository,
    V: VoiceService,
{
    unit_of_work: Arc<W>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<W, C, V> KickMember<W, C, V>
where
    W: ServerUnitOfWork,
    C: ChannelRepository,
    V: VoiceService,
{
    pub fn new(unit_of_work: Arc<W>, channel_repository: Arc<C>, voice_service: Arc<V>) -> Self {
        Self {
            unit_of_work,
            channel_repository,
            voice_service,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        actor: UserId,
        target: UserId,
    ) -> Result<(), KickMemberError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        ModerationPolicy::kick(&snapshot, &actor, &target)?;

        let member_event = messaging_domain::server::events::ServerEvent::MemberLeft(
            messaging_domain::server::events::lifecycle::ServerMemberLeft::record(
                server_id,
                target,
                chrono::Utc::now(),
            ),
        );
        if !tx.remove_member(&target).await? {
            return Err(KickMemberError::NotMember);
        }
        tx.save_outbox(&[member_event.to_outbox_record()?]).await?;
        tx.commit().await?;

        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_server(&server_id)
            .await?;

        for channel_id in &voice_channel_ids {
            let mut last_err = None;
            for _ in 0..3 {
                match self.voice_service.remove_participant(channel_id, &target).await {
                    Ok(()) => {
                        last_err = None;
                        break;
                    }
                    Err(e) => {
                        last_err = Some(e);
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                }
            }
            if let Some(err) = last_err {
                tracing::warn!(
                    channel_id = %channel_id.uuid(),
                    user_id = %target.uuid(),
                    error = %err,
                    "Failed to disconnect LiveKit participant after kick; join guard will enforce",
                );
            }
        }

        Ok(())
    }
}
