use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use domain::stream::StreamCeiling;
use ports::repository::RepositoryError;
use uuid::Uuid;
use voice_domain::stream::StreamLimited;

use crate::ports::{
    plan_directory::PlanDirectory,
    voice_service::{VoiceService, VoiceServiceError},
    webhook_outbox::WebhookOutbox,
};

#[derive(Debug, thiserror::Error)]
pub enum SweepScreenSharesError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
}

pub struct SweepScreenShares<P, V, O>
where
    P: PlanDirectory,
    V: VoiceService,
    O: WebhookOutbox,
{
    plans: Arc<P>,
    voice: Arc<V>,
    outbox: Arc<O>,
    suspects: Mutex<HashSet<(Uuid, Uuid, String)>>,
}

impl<P, V, O> SweepScreenShares<P, V, O>
where
    P: PlanDirectory,
    V: VoiceService,
    O: WebhookOutbox,
{
    pub fn new(plans: Arc<P>, voice: Arc<V>, outbox: Arc<O>) -> Self {
        Self { plans, voice, outbox, suspects: Mutex::new(HashSet::new()) }
    }

    pub async fn execute(&self, now: DateTime<Utc>) -> Result<usize, SweepScreenSharesError> {
        let previous = std::mem::take(&mut *self.suspects.lock().unwrap_or_else(|e| e.into_inner()));
        let mut current = HashSet::new();
        let mut limited = 0;
        for room in self.voice.active_rooms().await? {
            let shares = match self.voice.room_screen_shares(&room).await {
                Ok(shares) => shares,
                Err(e) => {
                    tracing::warn!(error = %e, room = %room.uuid(), "Failed to inspect screen shares of a room");
                    continue;
                }
            };
            for (user, video) in shares {
                let ceiling = StreamCeiling::for_tier(self.plans.tier_of(&user, now).await?);
                if ceiling.allows(video.width, video.height, video.bitrate) {
                    continue;
                }
                let key = (room.uuid(), user.uuid(), video.sid.clone());
                if !previous.contains(&key) {
                    current.insert(key);
                    continue;
                }
                let event = StreamLimited {
                    user_id: user,
                    room,
                    track_sid: video.sid.clone(),
                    ceiling,
                    occurred_at: now,
                };
                if let Err(e) = self.outbox.record_stream_limited(&event).await {
                    tracing::warn!(error = %e, user_id = %user.uuid(), "Failed to record a stream limit; will retry");
                    current.insert(key);
                    continue;
                }
                tracing::info!(
                    user_id = %user.uuid(),
                    room = %room.uuid(),
                    width = video.width,
                    height = video.height,
                    bitrate = video.bitrate,
                    "Screen share stayed above the plan ceiling; muting it"
                );
                if let Err(e) = self.voice.mute_track(&room, &user, &video.sid).await {
                    tracing::warn!(error = %e, user_id = %user.uuid(), "Failed to mute a screen share above the plan ceiling");
                }
                limited += 1;
            }
        }
        *self.suspects.lock().unwrap_or_else(|e| e.into_inner()) = current;
        Ok(limited)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use domain::{
        channel::{id::ChannelId, token::ChannelToken},
        community::id::CommunityId,
        plan::Tier,
        user::{id::UserId, name::UserName},
    };
    use ports::voice_service::{ParticipantAccess, PublishedVideo};
    use voice_domain::webhook::WebhookEvent;

    use super::*;

    struct Plans(HashMap<Uuid, Tier>);

    #[async_trait]
    impl PlanDirectory for Plans {
        async fn apply_tier(&self, _: &UserId, _: Tier, _: Option<DateTime<Utc>>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn tier_of(&self, user_id: &UserId, _: DateTime<Utc>) -> Result<Tier, RepositoryError> {
            Ok(self.0.get(&user_id.uuid()).copied().unwrap_or(Tier::Free))
        }
    }

    struct Rooms {
        room: ChannelId,
        shares: Vec<(UserId, PublishedVideo)>,
        muted: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl VoiceService for Rooms {
        async fn create_token(&self, _: &UserId, _: &UserName, _: &ChannelId, _: StreamCeiling) -> Result<ChannelToken, VoiceServiceError> {
            unimplemented!()
        }
        async fn create_voice_channel(&self, _: &ChannelId, _: &CommunityId) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn delete_voice_channel(&self, _: &ChannelId) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn remove_participant(&self, _: &ChannelId, _: &UserId) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn create_token_with_access(
            &self,
            _: &UserId,
            _: &UserName,
            _: &ChannelId,
            _: ParticipantAccess,
            _: StreamCeiling,
        ) -> Result<ChannelToken, VoiceServiceError> {
            unimplemented!()
        }
        async fn apply_participant_access(&self, _: &ChannelId, _: &UserId, _: ParticipantAccess) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn set_stream_ceiling(&self, _: &ChannelId, _: &UserId, _: StreamCeiling) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn screen_shares(&self, _: &ChannelId, _: &UserId) -> Result<Vec<PublishedVideo>, VoiceServiceError> {
            unimplemented!()
        }
        async fn mute_track(&self, _: &ChannelId, _: &UserId, track_sid: &str) -> Result<(), VoiceServiceError> {
            self.muted.lock().unwrap().push(track_sid.to_string());
            Ok(())
        }
        async fn active_rooms(&self) -> Result<Vec<ChannelId>, VoiceServiceError> {
            Ok(vec![self.room])
        }
        async fn room_screen_shares(&self, _: &ChannelId) -> Result<Vec<(UserId, PublishedVideo)>, VoiceServiceError> {
            Ok(self.shares.clone())
        }
        fn client_url(&self) -> String {
            String::new()
        }
    }

    #[derive(Default)]
    struct Limits(Mutex<Vec<StreamLimited>>);

    #[async_trait]
    impl WebhookOutbox for Limits {
        async fn save(&self, _: &WebhookEvent, _: Option<&str>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn record_stream_limited(&self, event: &StreamLimited) -> Result<(), RepositoryError> {
            self.0.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    fn video(sid: &str, width: u32, height: u32) -> PublishedVideo {
        PublishedVideo { sid: sid.into(), width, height, bitrate: 1_000_000 }
    }

    #[tokio::test]
    async fn mutes_a_share_that_stays_above_the_plan_for_two_sweeps() {
        let (free, premium) = (Uuid::new_v4(), Uuid::new_v4());
        let voice = Arc::new(Rooms {
            room: ChannelId::new(Uuid::new_v4()),
            shares: vec![
                (UserId::new(free), video("free-4k", 3840, 2160)),
                (UserId::new(free), video("free-720", 1280, 720)),
                (UserId::new(premium), video("premium-4k", 3840, 2160)),
            ],
            muted: Mutex::new(Vec::new()),
        });
        let limits = Arc::new(Limits::default());
        let sweep = SweepScreenShares::new(
            Arc::new(Plans(HashMap::from([(premium, Tier::Premium)]))),
            voice.clone(),
            limits.clone(),
        );
        assert_eq!(sweep.execute(Utc::now()).await.unwrap(), 0);
        assert!(voice.muted.lock().unwrap().is_empty());
        assert_eq!(sweep.execute(Utc::now()).await.unwrap(), 1);
        assert_eq!(*voice.muted.lock().unwrap(), vec!["free-4k".to_string()]);
        let limits = limits.0.lock().unwrap();
        assert_eq!(limits.len(), 1);
        assert_eq!(limits[0].user_id, UserId::new(free));
        assert_eq!(limits[0].track_sid, "free-4k");
        assert_eq!(limits[0].ceiling, StreamCeiling::FREE);
    }
}
