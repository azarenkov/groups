use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{channel::repository::ChannelRepository, events::MessagingEvent, community::events::CommunityEvent, community::events::lifecycle::CommunityDeleted, community::unit_of_work::CommunityUnitOfWork};
use ports::repository::RepositoryError;

use crate::ports::voice_service::VoiceService;

#[derive(Debug, thiserror::Error)]
pub enum ResolveDeletedUserCommunitiesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ResolveDeletedUserCommunities<C, W, V>
where
    C: ChannelRepository,
    W: CommunityUnitOfWork,
    V: VoiceService,
{
    channel_repository: Arc<C>,
    unit_of_work: Arc<W>,
    voice_service: Arc<V>,
}

impl<C, W, V> ResolveDeletedUserCommunities<C, W, V>
where
    C: ChannelRepository,
    W: CommunityUnitOfWork,
    V: VoiceService,
{
    pub fn new(
        channel_repository: Arc<C>,
        unit_of_work: Arc<W>,
        voice_service: Arc<V>,
    ) -> Self {
        Self {
            channel_repository,
            unit_of_work,
            voice_service,
        }
    }

    pub async fn execute(&self, user_id: UserId, deleted_at: chrono::DateTime<Utc>, request_id: Option<&str>) -> Result<(), ResolveDeletedUserCommunitiesError> {
        let owned = {
            let mut user_tx = self.unit_of_work.begin_user(&user_id).await?;
            user_tx.mark_user_deleted(deleted_at).await?;
            let owned = user_tx.owned_community_ids().await?;
            user_tx.commit().await?;
            owned
        };
        for community_id in owned {
            let voice_channel_ids = self
                .channel_repository
                .list_voice_channel_ids_by_community(&community_id)
                .await?;

            let mut tx = self.unit_of_work.begin(&community_id).await?;
            tx.lock_user(&user_id).await?;
            let mut community = match tx.load_community().await {
                Ok(community) => community,
                Err(RepositoryError::NotFound) => continue,
                Err(e) => return Err(e.into()),
            };
            if !community.is_owner(&user_id) {
                continue;
            }
            let now = Utc::now();
            let mut successor = None;
            for _ in 0..8 {
                let Some(candidate) = tx.find_longest_member_excluding(&user_id).await? else {
                    break;
                };
                tx.lock_user(&candidate).await?;
                if tx.is_user_deleted(&candidate).await? {
                    continue;
                }
                successor = Some(candidate);
                break;
            }
            let deleted_rooms = match successor.filter(|s| community.is_member(s)) {
                Some(new_owner) => {
                    if community.transfer_ownership(new_owner, now).is_err() {
                        continue;
                    }
                    tx.update_community_owner(&new_owner).await?;
                    let events: Vec<MessagingEvent> =
                        community.pull_events().into_iter().map(MessagingEvent::from).collect();
                    tx.save_events(&events, request_id).await?;
                    tx.commit().await?;
                    tracing::info!(community_id = %community_id.uuid(), previous_owner = %user_id.uuid(), new_owner = %new_owner.uuid(), "Transferred community ownership from deleted user");
                    false
                }
                None => {
                    let member_ids = community.member_ids();
                    let event = CommunityEvent::Deleted(CommunityDeleted::record(community_id, user_id, member_ids, now));
                    if tx.delete_community_by_owner(&user_id).await?.is_none() {
                        continue;
                    }
                    tx.save_events(&[MessagingEvent::from(event)], request_id).await?;
                    tx.commit().await?;
                    tracing::info!(community_id = %community_id.uuid(), owner = %user_id.uuid(), "Deleted community left without members by deleted user");
                    true
                }
            };
            if deleted_rooms {
                for channel_id in &voice_channel_ids {
                    if let Err(err) = self.voice_service.delete_voice_channel(channel_id).await {
                        tracing::warn!(channel_id = %channel_id.uuid(), community_id = %community_id.uuid(), error = %err, "Failed to delete LiveKit room after deleting community");
                    }
                }
            }
        }
        Ok(())
    }
}
