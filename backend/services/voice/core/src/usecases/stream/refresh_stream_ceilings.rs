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
    use std::{
        collections::HashMap,
        sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };

    use domain::{
        channel::{id::ChannelId, token::ChannelToken},
        community::id::CommunityId,
        plan::Tier,
        user::name::UserName,
    };
    use ports::voice_service::{ParticipantAccess, PublishedVideo};
    use uuid::Uuid;

    use super::*;

    struct Plans(HashMap<Uuid, Tier>, AtomicBool);

    #[async_trait]
    impl PlanDirectory for Plans {
        async fn apply_tier(&self, _: &UserId, _: Tier, _: Option<DateTime<Utc>>, _: DateTime<Utc>) -> Result<(), RepositoryError> {
            unimplemented!()
        }
        async fn tier_of(&self, user_id: &UserId, _: DateTime<Utc>) -> Result<Tier, RepositoryError> {
            Ok(self.0.get(&user_id.uuid()).copied().unwrap_or(Tier::Free))
        }
        fn is_caught_up(&self) -> bool {
            self.1.load(Ordering::SeqCst)
        }
        fn set_caught_up(&self, caught_up: bool) {
            self.1.store(caught_up, Ordering::SeqCst);
        }
    }

    struct Rooms {
        rooms: Vec<(ChannelId, Vec<UserId>)>,
        ceilings: Mutex<Vec<(ChannelId, UserId, StreamCeiling)>>,
    }

    #[async_trait]
    impl VoiceService for Rooms {
        async fn create_token(&self, _: &UserId, _: &UserName, _: &ChannelId) -> Result<ChannelToken, VoiceServiceError> {
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
        ) -> Result<ChannelToken, VoiceServiceError> {
            unimplemented!()
        }
        async fn apply_participant_access(&self, _: &ChannelId, _: &UserId, _: ParticipantAccess) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn set_stream_ceiling(&self, room: &ChannelId, user: &UserId, ceiling: StreamCeiling) -> Result<(), VoiceServiceError> {
            self.ceilings.lock().unwrap().push((*room, *user, ceiling));
            Ok(())
        }
        async fn mute_track(&self, _: &ChannelId, _: &UserId, _: &str) -> Result<(), VoiceServiceError> {
            unimplemented!()
        }
        async fn active_rooms(&self) -> Result<Vec<ChannelId>, VoiceServiceError> {
            Ok(self.rooms.iter().map(|(room, _)| *room).collect())
        }
        async fn room_participants(&self, room: &ChannelId) -> Result<Vec<UserId>, VoiceServiceError> {
            Ok(self.rooms.iter().find(|(id, _)| id == room).map(|(_, users)| users.clone()).unwrap_or_default())
        }
        async fn room_videos(&self, _: &ChannelId) -> Result<Vec<(UserId, PublishedVideo)>, VoiceServiceError> {
            unimplemented!()
        }
        fn client_url(&self) -> String {
            String::new()
        }
    }

    fn setup(caught_up: bool) -> (RefreshStreamCeilings<Plans, Rooms>, Arc<Rooms>, [UserId; 2], [ChannelId; 2]) {
        let (pro, free) = (UserId::new(Uuid::new_v4()), UserId::new(Uuid::new_v4()));
        let (call, channel) = (ChannelId::new(Uuid::new_v4()), ChannelId::new(Uuid::new_v4()));
        let rooms = Arc::new(Rooms {
            rooms: vec![(call, vec![pro, free]), (channel, vec![pro])],
            ceilings: Mutex::default(),
        });
        let plans = Arc::new(Plans(HashMap::from([(pro.uuid(), Tier::Pro)]), AtomicBool::new(caught_up)));
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
