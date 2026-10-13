use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use domain::{channel::id::ChannelId, plan::Tier, stream::StreamCeiling, user::id::UserId};
use ports::repository::RepositoryError;
use voice_domain::call::repository::CallRepository;

use crate::ports::{
    plan_directory::PlanDirectory,
    voice_service::{VoiceService, VoiceServiceError},
};

const LIVE_CHANGE_WINDOW_HOURS: i64 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ApplyPlanChangeError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Voice service error: {0}")]
    Voice(#[from] VoiceServiceError),
}

pub struct PlanChange {
    pub user_id: UserId,
    pub tier: Tier,
    pub valid_until: Option<DateTime<Utc>>,
    pub occurred_at: DateTime<Utc>,
}

pub struct ApplyPlanChange<P, C, V>
where
    P: PlanDirectory,
    C: CallRepository,
    V: VoiceService,
{
    plans: Arc<P>,
    calls: Arc<C>,
    voice: Arc<V>,
}

impl<P, C, V> ApplyPlanChange<P, C, V>
where
    P: PlanDirectory,
    C: CallRepository,
    V: VoiceService,
{
    pub fn new(plans: Arc<P>, calls: Arc<C>, voice: Arc<V>) -> Self {
        Self { plans, calls, voice }
    }

    pub async fn execute(&self, change: PlanChange, now: DateTime<Utc>) -> Result<(), ApplyPlanChangeError> {
        self.plans
            .apply_tier(&change.user_id, change.tier, change.valid_until, change.occurred_at)
            .await?;
        if now - change.occurred_at > Duration::hours(LIVE_CHANGE_WINDOW_HOURS) {
            return Ok(());
        }
        let Some(call_id) = self.calls.active_call_for(&change.user_id).await? else {
            return Ok(());
        };
        let ceiling = StreamCeiling::for_tier(self.plans.tier_of(&change.user_id, now).await?);
        self.voice
            .set_stream_ceiling(&ChannelId::new(call_id), &change.user_id, ceiling)
            .await?;
        Ok(())
    }
}
