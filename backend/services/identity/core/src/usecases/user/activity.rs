use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use domain::{event::EventId, user::id::UserId};
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityUnitOfWork,
    user::events::UserEvent, user::events::lifecycle::UserActivityChanged,
};
use ports::repository::RepositoryError;

use crate::ports::activity_audience::{ActivityAudience, ActivityAudienceError};
use crate::ports::activity_store::{ActivityStore, ActivityStoreError, StoredActivity};

pub const ACTIVITY_TTL: Duration = Duration::from_secs(60);
const MAX_BATCH: usize = 100;

#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Activity store error: {0}")]
    Store(#[from] ActivityStoreError),
    #[error("Activity audience error: {0}")]
    Audience(#[from] ActivityAudienceError),
    #[error("Invalid activity")]
    Invalid,
    #[error("Activity is hidden by privacy settings")]
    Hidden,
    #[error("User not found")]
    UserNotFound,
}

pub struct ActivityInput {
    pub game_id: String,
    pub name: String,
    pub started_at: DateTime<Utc>,
}

fn valid_game_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn valid_name(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && trimmed.chars().count() <= 64
}

fn activity_event(user_id: UserId, activity: Option<&StoredActivity>) -> IdentityEvent {
    IdentityEvent::from(UserEvent::ActivityChanged(UserActivityChanged {
        id: EventId::generate(),
        user_id,
        game_id: activity.map(|a| a.game_id.clone()),
        name: activity.map(|a| a.name.clone()),
        started_at: activity.map(|a| a.started_at),
        occurred_at: Utc::now(),
    }))
}

pub struct SetActivity<S: ActivityStore, W: IdentityUnitOfWork> {
    store: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<S: ActivityStore, W: IdentityUnitOfWork> SetActivity<S, W> {
    pub fn new(store: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self {
            store,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        input: ActivityInput,
    ) -> Result<StoredActivity, ActivityError> {
        if !valid_game_id(&input.game_id)
            || !valid_name(&input.name)
            || input.started_at > Utc::now() + chrono::Duration::minutes(5)
        {
            return Err(ActivityError::Invalid);
        }
        let mut tx = self.unit_of_work.begin().await?;
        let user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(ActivityError::UserNotFound)?;
        if !user.show_game_activity() {
            self.store.clear(&user_id).await?;
            tx.save_events(&[activity_event(user_id, None)], None).await?;
            tx.commit().await?;
            return Err(ActivityError::Hidden);
        }
        let previous = self.store.get(&user_id).await?;
        let needs_publish = previous
            .as_ref()
            .map(|p| p.game_id != input.game_id || !p.published)
            .unwrap_or(true);
        let mut activity = StoredActivity {
            game_id: input.game_id,
            name: input.name.trim().to_string(),
            started_at: input.started_at,
            published: !needs_publish,
        };
        self.store.set(&user_id, &activity, ACTIVITY_TTL).await?;
        if needs_publish {
            tx.save_events(&[activity_event(user_id, Some(&activity))], None).await?;
        }
        tx.commit().await?;
        if needs_publish {
            activity.published = true;
            self.store.set(&user_id, &activity, ACTIVITY_TTL).await?;
        }
        Ok(activity)
    }
}

pub struct ClearActivity<S: ActivityStore, W: IdentityUnitOfWork> {
    store: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<S: ActivityStore, W: IdentityUnitOfWork> ClearActivity<S, W> {
    pub fn new(store: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self {
            store,
            unit_of_work,
        }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<(), ActivityError> {
        let mut tx = self.unit_of_work.begin().await?;
        if tx.load_user_for_update(&user_id).await?.is_none() {
            return Err(ActivityError::UserNotFound);
        }
        self.store.clear(&user_id).await?;
        tx.save_events(&[activity_event(user_id, None)], None).await?;
        tx.commit().await?;
        Ok(())
    }
}

pub struct GetActivities<S: ActivityStore, A: ActivityAudience> {
    store: Arc<S>,
    audience: Arc<A>,
}

impl<S: ActivityStore, A: ActivityAudience> GetActivities<S, A> {
    pub fn new(store: Arc<S>, audience: Arc<A>) -> Self {
        Self { store, audience }
    }

    pub async fn execute(
        &self,
        viewer: UserId,
        user_ids: Vec<UserId>,
    ) -> Result<Vec<(UserId, StoredActivity)>, ActivityError> {
        if user_ids.len() > MAX_BATCH {
            return Err(ActivityError::Invalid);
        }
        let visible = self.audience.visible_to(&viewer, &user_ids).await?;
        if visible.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.store.get_many(&visible).await?)
    }
}
