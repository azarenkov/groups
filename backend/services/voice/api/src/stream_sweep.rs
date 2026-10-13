use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use domain::stream::StreamCeiling;
use ports::user_profile::UserProfileRepository;
use ports::voice_service::stream_limited_record;
use uuid::Uuid;
use voice::ports::voice_service::VoiceService;
use voice::ports::webhook_outbox::WebhookOutbox;

const SWEEP_EVERY: Duration = Duration::from_secs(30);

pub fn spawn<P, V, O>(profiles: Arc<P>, voice: Arc<V>, outbox: Arc<O>)
where
    P: UserProfileRepository + Send + Sync + 'static,
    V: VoiceService + Send + Sync + 'static,
    O: WebhookOutbox + 'static,
{
    actix_web::rt::spawn(async move {
        let sweep = ShareSweep::default();
        loop {
            actix_web::rt::time::sleep(SWEEP_EVERY).await;
            if let Err(e) = sweep.run(&*profiles, &*voice, &*outbox).await {
                tracing::warn!(error = %e, "Screen share sweep failed");
            }
        }
    });
}

#[derive(Default)]
pub struct ShareSweep {
    suspects: Mutex<HashSet<(Uuid, Uuid, String)>>,
}

impl ShareSweep {
    pub async fn run<P, V, O>(&self, profiles: &P, voice: &V, outbox: &O) -> Result<usize, Box<dyn std::error::Error + Send + Sync>>
    where
        P: UserProfileRepository + Send + Sync,
        V: VoiceService + Send + Sync,
        O: WebhookOutbox,
    {
        let previous = std::mem::take(&mut *self.suspects.lock().unwrap_or_else(|e| e.into_inner()));
        let mut current = HashSet::new();
        let mut limited = 0;
        for room in voice.active_rooms().await? {
            let shares = match voice.room_screen_shares(&room).await {
                Ok(shares) => shares,
                Err(e) => {
                    tracing::warn!(error = %e, room = %room.uuid(), "Failed to inspect screen shares of a room");
                    continue;
                }
            };
            for (user, video) in shares {
                let ceiling = StreamCeiling::for_tier(&profiles.tier_of(&user, chrono::Utc::now()).await?);
                if ceiling.allows(video.width, video.height, video.bitrate) {
                    continue;
                }
                let key = (room.uuid(), user.uuid(), video.sid.clone());
                if !previous.contains(&key) {
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
                if let Err(e) = outbox.save_record(stream_limited_record(&user, &room, &video.sid, ceiling)).await {
                    tracing::warn!(error = %e, user_id = %user.uuid(), "Failed to record a stream limit; will retry");
                    current.insert(key);
                    continue;
                }
                if let Err(e) = voice.mute_track(&room, &user, &video.sid).await {
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
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use domain::channel::id::ChannelId;
    use domain::channel::token::ChannelToken;
    use domain::community::id::CommunityId;
    use domain::user::{id::UserId, name::UserName};
    use ports::outbox::record::OutboxRecord;
    use ports::repository::RepositoryError;
    use ports::user_profile::PublicProfile;
    use ports::voice_service::{ParticipantAccess, PublishedVideo, VoiceServiceError};
    use uuid::Uuid;
    use voice_domain::webhook::WebhookEvent;

    use super::*;

    struct Tiers(HashMap<Uuid, &'static str>);

    #[async_trait]
    impl UserProfileRepository for Tiers {
        async fn get_public_profile(&self, _: &UserId) -> Result<Option<PublicProfile>, RepositoryError> {
            unimplemented!()
        }
        async fn update_profile(&self, _: &UserId, _: &str, _: bool, _: bool, _: &str, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn set_registered_at(&self, _: &UserId, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn get_name(&self, _: &UserId) -> Result<Option<String>, RepositoryError> {
            unimplemented!()
        }
        async fn find_user_by_name(&self, _: &str) -> Result<Option<UserId>, RepositoryError> {
            unimplemented!()
        }
        async fn upsert_name(&self, _: &UserId, _: &str, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn is_active(&self, _: &UserId) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn is_deactivation_current(&self, _: &UserId, _: DateTime<Utc>) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn set_active(&self, _: &UserId, _: bool, _: DateTime<Utc>) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
        async fn apply_tier(&self, _: &UserId, _: &str, _: Option<DateTime<Utc>>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn tier_of(&self, user_id: &UserId, _: DateTime<Utc>) -> Result<String, RepositoryError> {
            Ok(self.0.get(&user_id.uuid()).copied().unwrap_or("free").to_string())
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
    struct Records(Mutex<Vec<OutboxRecord>>);

    #[async_trait]
    impl WebhookOutbox for Records {
        async fn save(&self, _: &WebhookEvent, _: Option<&str>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn save_record(&self, record: OutboxRecord) -> Result<(), RepositoryError> {
            self.0.lock().unwrap().push(record);
            Ok(())
        }
    }

    fn video(sid: &str, width: u32, height: u32) -> PublishedVideo {
        PublishedVideo { sid: sid.into(), width, height, bitrate: 1_000_000 }
    }

    #[tokio::test]
    async fn mutes_a_share_that_stays_above_the_plan_for_two_sweeps() {
        let (free, premium) = (Uuid::new_v4(), Uuid::new_v4());
        let profiles = Tiers(HashMap::from([(premium, "premium")]));
        let voice = Rooms {
            room: ChannelId::new(Uuid::new_v4()),
            shares: vec![
                (UserId::new(free), video("free-4k", 3840, 2160)),
                (UserId::new(free), video("free-720", 1280, 720)),
                (UserId::new(premium), video("premium-4k", 3840, 2160)),
            ],
            muted: Mutex::new(Vec::new()),
        };
        let outbox = Records::default();
        let sweep = ShareSweep::default();
        assert_eq!(sweep.run(&profiles, &voice, &outbox).await.unwrap(), 0);
        assert!(voice.muted.lock().unwrap().is_empty());
        assert_eq!(sweep.run(&profiles, &voice, &outbox).await.unwrap(), 1);
        assert_eq!(*voice.muted.lock().unwrap(), vec!["free-4k".to_string()]);
        let records = outbox.0.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].aggregate_id, free);
        assert_eq!(records[0].event_type, "stream_limited");
    }
}
