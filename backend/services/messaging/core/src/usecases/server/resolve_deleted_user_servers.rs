use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{channel::repository::ChannelRepository, events::MessagingEvent, server::events::ServerEvent, server::events::lifecycle::ServerDeleted, server::repository::ServerRepository, server::unit_of_work::ServerUnitOfWork};
use ports::repository::RepositoryError;

use crate::ports::voice_service::VoiceService;

#[derive(Debug, thiserror::Error)]
pub enum ResolveDeletedUserServersError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ResolveDeletedUserServers<S, C, W, V>
where
    S: ServerRepository,
    C: ChannelRepository,
    W: ServerUnitOfWork,
    V: VoiceService,
{
    server_repository: Arc<S>,
    channel_repository: Arc<C>,
    unit_of_work: Arc<W>,
    voice_service: Arc<V>,
}

impl<S, C, W, V> ResolveDeletedUserServers<S, C, W, V>
where
    S: ServerRepository,
    C: ChannelRepository,
    W: ServerUnitOfWork,
    V: VoiceService,
{
    pub fn new(
        server_repository: Arc<S>,
        channel_repository: Arc<C>,
        unit_of_work: Arc<W>,
        voice_service: Arc<V>,
    ) -> Self {
        Self {
            server_repository,
            channel_repository,
            unit_of_work,
            voice_service,
        }
    }

    pub async fn execute(&self, user_id: UserId, request_id: Option<&str>) -> Result<(), ResolveDeletedUserServersError> {
        let owned = self.server_repository.find_server_ids_owned_by(&user_id).await?;
        for server_id in owned {
            let successor = self
                .server_repository
                .find_longest_member_excluding(&server_id, &user_id)
                .await?;
            let voice_channel_ids = self
                .channel_repository
                .list_voice_channel_ids_by_server(&server_id)
                .await?;

            let mut tx = self.unit_of_work.begin(&server_id).await?;
            let mut server = match tx.load_server().await {
                Ok(server) => server,
                Err(RepositoryError::NotFound) => continue,
                Err(e) => return Err(e.into()),
            };
            if !server.is_owner(&user_id) {
                continue;
            }
            let now = Utc::now();
            let deleted_rooms = match successor.filter(|s| server.is_member(s)) {
                Some(new_owner) => {
                    if server.transfer_ownership(new_owner, now).is_err() {
                        continue;
                    }
                    tx.update_server_owner(&new_owner).await?;
                    let events: Vec<MessagingEvent> =
                        server.pull_events().into_iter().map(MessagingEvent::from).collect();
                    tx.save_events(&events, request_id).await?;
                    tx.commit().await?;
                    tracing::info!(server_id = %server_id.uuid(), previous_owner = %user_id.uuid(), new_owner = %new_owner.uuid(), "Transferred server ownership from deleted user");
                    false
                }
                None => {
                    let member_ids = server.member_ids();
                    let event = ServerEvent::Deleted(ServerDeleted::record(server_id, user_id, member_ids, now));
                    if tx.delete_server_by_owner(&user_id).await?.is_none() {
                        continue;
                    }
                    tx.save_events(&[MessagingEvent::from(event)], request_id).await?;
                    tx.commit().await?;
                    tracing::info!(server_id = %server_id.uuid(), owner = %user_id.uuid(), "Deleted server left without members by deleted user");
                    true
                }
            };
            if deleted_rooms {
                for channel_id in &voice_channel_ids {
                    if let Err(err) = self.voice_service.delete_voice_channel(channel_id).await {
                        tracing::warn!(channel_id = %channel_id.uuid(), server_id = %server_id.uuid(), error = %err, "Failed to delete LiveKit room after deleting server");
                    }
                }
            }
        }
        Ok(())
    }
}
