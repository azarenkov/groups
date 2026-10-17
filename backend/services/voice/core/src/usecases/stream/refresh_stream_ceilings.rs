use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{stream::StreamCeiling, user::id::UserId};
use ports::repository::RepositoryError;

use crate::ports::{
    plan_directory::{PlanDirectory, PlanListener, PlanListenerError},
    voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum RefreshStreamCeilingsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
}

pub struct RefreshStreamCeilings<P, V>
where
    P: PlanDirectory,
    V: VoiceService,
{
    plans: Arc<P>,
    voice: Arc<V>,
}

impl<P, V> RefreshStreamCeilings<P, V>
where
    P: PlanDirectory,
    V: VoiceService + Send + Sync,
{
    pub fn new(plans: Arc<P>, voice: Arc<V>) -> Self {
        Self { plans, voice }
    }

    pub async fn for_user(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<(), RefreshStreamCeilingsError> {
        let ceiling = StreamCeiling::for_tier(self.plans.tier_of(user_id, now).await?);
        for room in self.voice.active_rooms().await? {
            if self.voice.room_participants(&room).await?.contains(user_id) {
                self.voice.set_stream_ceiling(&room, user_id, ceiling).await?;
            }
        }
        Ok(())
    }

    pub async fn for_everyone(&self, now: DateTime<Utc>) -> Result<(), RefreshStreamCeilingsError> {
        for room in self.voice.active_rooms().await? {
            for user in self.voice.room_participants(&room).await? {
                let ceiling = StreamCeiling::for_tier(self.plans.tier_of(&user, now).await?);
                self.voice.set_stream_ceiling(&room, &user, ceiling).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<P, V> PlanListener for RefreshStreamCeilings<P, V>
where
    P: PlanDirectory,
    V: VoiceService + Send + Sync,
{
    async fn plan_applied(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<(), PlanListenerError> {
        if !self.plans.is_caught_up() {
            return Ok(());
        }
        Ok(self.for_user(user_id, now).await?)
    }

    async fn caught_up(&self, now: DateTime<Utc>) -> Result<(), PlanListenerError> {
        Ok(self.for_everyone(now).await?)
    }
}

#[cfg(test)]
mod tests {
    use domain::{channel::id::ChannelId, plan::Tier};
    use ports::fakes::{FakePlanDirectory, FakeVoiceService};
    use uuid::Uuid;

    use super::*;

    fn setup(
        caught_up: bool,
    ) -> (RefreshStreamCeilings<FakePlanDirectory, FakeVoiceService>, Arc<FakeVoiceService>, [UserId; 2], [ChannelId; 2]) {
        let (pro, free) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let (call, channel) = (ChannelId::new(Uuid::new_v4()), ChannelId::new(Uuid::new_v4()));
        let rooms = Arc::new(FakeVoiceService::default().room(call, vec![pro, free]).room(channel, vec![pro]));
        let plans = Arc::new(FakePlanDirectory::new([(pro.uuid(), Tier::Pro)]).caught_up(caught_up));
        (RefreshStreamCeilings::new(plans, rooms.clone()), rooms, [pro, free], [call, channel])
    }

    #[tokio::test]
    async fn a_plan_change_waits_for_the_projection_to_catch_up() {
        let (usecase, rooms, [pro, _], _) = setup(false);
        usecase.plan_applied(&pro, Utc::now()).await.unwrap();
        assert!(rooms.ceilings.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_plan_change_reaches_every_room_the_user_is_in() {
        let (usecase, rooms, [pro, _], [call, channel]) = setup(true);
        usecase.plan_applied(&pro, Utc::now()).await.unwrap();
        assert_eq!(
            *rooms.ceilings.lock().unwrap(),
            vec![(call, pro, StreamCeiling::PRO), (channel, pro, StreamCeiling::PRO)]
        );
    }

    #[tokio::test]
    async fn catching_up_refreshes_every_participant_of_every_room() {
        let (usecase, rooms, [pro, free], [call, channel]) = setup(false);
        usecase.caught_up(Utc::now()).await.unwrap();
        assert_eq!(
            *rooms.ceilings.lock().unwrap(),
            vec![
                (call, pro, StreamCeiling::PRO),
                (call, free, StreamCeiling::FREE),
                (channel, pro, StreamCeiling::PRO),
            ]
        );
    }
}
