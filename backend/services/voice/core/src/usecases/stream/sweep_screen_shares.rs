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
    suspects: Mutex<HashSet<(Uuid, Uuid)>>,
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
        if !self.plans.is_caught_up() {
            self.suspects.lock().unwrap_or_else(|e| e.into_inner()).clear();
            return Ok(0);
        }
        let previous = self.suspects.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let rooms = self.voice.active_rooms().await?;
        let mut current = HashSet::new();
        let mut limited = 0;
        for room in rooms {
            let shares = match self.voice.room_videos(&room).await {
                Ok(shares) => shares,
                Err(e) => {
                    tracing::warn!(error = %e, room = %room.uuid(), "Failed to inspect screen shares of a room");
                    current.extend(previous.iter().filter(|(r, _)| *r == room.uuid()).copied());
                    continue;
                }
            };
            for (user, video) in shares {
                let key = (room.uuid(), user.uuid());
                let ceiling = match self.plans.tier_of(&user, now).await {
                    Ok(tier) => StreamCeiling::for_tier(tier),
                    Err(e) => {
                        tracing::warn!(error = %e, user_id = %user.uuid(), "Failed to read a plan during the sweep");
                        if previous.contains(&key) {
                            current.insert(key);
                        }
                        continue;
                    }
                };
                if ceiling.allows(video.width, video.height, video.bitrate) {
                    continue;
                }
                current.insert(key);
                if !previous.contains(&key) {
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
    use async_trait::async_trait;
    use domain::{channel::id::ChannelId, plan::Tier, user::id::UserId};
    use ports::fakes::{FakePlanDirectory, FakeVoiceService};
    use ports::voice_service::PublishedVideo;
    use voice_domain::webhook::WebhookEvent;

    use super::*;

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
    async fn keeps_strikes_when_the_rooms_cannot_be_listed() {
        let free = Uuid::new_v4();
        let room = ChannelId::new(Uuid::new_v4());
        let voice = Arc::new(FakeVoiceService::default().room(room, Vec::new()).videos(room, vec![(UserId::new(free), video("big", 3840, 2160))]));
        let sweep = SweepScreenShares::new(Arc::new(FakePlanDirectory::new([])), voice.clone(), Arc::new(Limits::default()));
        sweep.execute(Utc::now()).await.unwrap();
        let broken = Arc::new(FakeVoiceService::unreachable());
        let failing = SweepScreenShares::new(Arc::new(FakePlanDirectory::new([])), broken, Arc::new(Limits::default()));
        *failing.suspects.lock().unwrap() = sweep.suspects.lock().unwrap().clone();
        assert!(failing.execute(Utc::now()).await.is_err());
        assert!(failing.suspects.lock().unwrap().contains(&(room.uuid(), free)));
    }

    #[tokio::test]
    async fn keeps_the_strike_when_an_over_limit_share_is_republished() {
        let free = Uuid::new_v4();
        let room = ChannelId::new(Uuid::new_v4());
        let limits = Arc::new(Limits::default());
        let plans = Arc::new(FakePlanDirectory::new([]));
        let first = Arc::new(FakeVoiceService::default().room(room, Vec::new()).videos(room, vec![(UserId::new(free), video("first", 3840, 2160))]));
        let sweep = SweepScreenShares::new(plans.clone(), first.clone(), limits.clone());
        assert_eq!(sweep.execute(Utc::now()).await.unwrap(), 0);
        let suspects = std::mem::take(&mut *sweep.suspects.lock().unwrap());
        let republished = Arc::new(FakeVoiceService::default().room(room, Vec::new()).videos(room, vec![(UserId::new(free), video("second", 3840, 2160))]));
        let sweep = SweepScreenShares::new(plans, republished.clone(), limits.clone());
        *sweep.suspects.lock().unwrap() = suspects;
        assert_eq!(sweep.execute(Utc::now()).await.unwrap(), 1);
        assert_eq!(*republished.muted.lock().unwrap(), vec!["second".to_string()]);
        assert_eq!(sweep.execute(Utc::now()).await.unwrap(), 1);
        assert_eq!(republished.muted.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn mutes_a_share_that_stays_above_the_plan_for_two_sweeps() {
        let (free, premium) = (Uuid::new_v4(), Uuid::new_v4());
        let room = ChannelId::new(Uuid::new_v4());
        let voice = Arc::new(FakeVoiceService::default().room(room, Vec::new()).videos(room, vec![
                (UserId::new(free), video("free-4k", 3840, 2160)),
                (UserId::new(free), video("free-720", 1280, 720)),
                (UserId::new(premium), video("premium-4k", 3840, 2160)),
            ]));
        let limits = Arc::new(Limits::default());
        let sweep = SweepScreenShares::new(
            Arc::new(FakePlanDirectory::new([(premium, Tier::Premium)])),
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
