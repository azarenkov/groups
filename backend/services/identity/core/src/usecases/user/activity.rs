use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use domain::{event::EventId, user::id::UserId};
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityUnitOfWork,
    user::events::UserEvent, user::events::lifecycle::UserActivityChanged,
    user::repository::UserRepository,
};
use ports::repository::RepositoryError;

use crate::ports::activity_store::{ActivityStore, ActivityStoreError, StoredActivity};

pub const ACTIVITY_TTL: Duration = Duration::from_secs(60);
const MAX_BATCH: usize = 100;

#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Activity store error: {0}")]
    Store(#[from] ActivityStoreError),
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

async fn publish<W: IdentityUnitOfWork>(
    unit_of_work: &W,
    user_id: UserId,
    activity: Option<&StoredActivity>,
) -> Result<(), ActivityError> {
    let event = UserEvent::ActivityChanged(UserActivityChanged {
        id: EventId::generate(),
        user_id,
        game_id: activity.map(|a| a.game_id.clone()),
        name: activity.map(|a| a.name.clone()),
        started_at: activity.map(|a| a.started_at),
        occurred_at: Utc::now(),
    });
    let mut tx = unit_of_work.begin().await?;
    tx.save_events(&[IdentityEvent::from(event)], None).await?;
    tx.commit().await?;
    Ok(())
}

pub struct SetActivity<U: UserRepository, S: ActivityStore, W: IdentityUnitOfWork> {
    user_repository: Arc<U>,
    store: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<U: UserRepository, S: ActivityStore, W: IdentityUnitOfWork> SetActivity<U, S, W> {
    pub fn new(user_repository: Arc<U>, store: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self {
            user_repository,
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
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(ActivityError::UserNotFound)?;
        if !user.show_game_activity() {
            if self.store.clear(&user_id).await?.is_some() {
                publish(self.unit_of_work.as_ref(), user_id, None).await?;
            }
            return Err(ActivityError::Hidden);
        }
        let activity = StoredActivity {
            game_id: input.game_id,
            name: input.name.trim().to_string(),
            started_at: input.started_at,
        };
        let previous = self.store.set(&user_id, &activity, ACTIVITY_TTL).await?;
        let changed = previous
            .as_ref()
            .map(|p| p.game_id != activity.game_id)
            .unwrap_or(true);
        if changed {
            publish(self.unit_of_work.as_ref(), user_id, Some(&activity)).await?;
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
        if self.store.clear(&user_id).await?.is_some() {
            publish(self.unit_of_work.as_ref(), user_id, None).await?;
        }
        Ok(())
    }
}

pub struct GetActivities<S: ActivityStore> {
    store: Arc<S>,
}

impl<S: ActivityStore> GetActivities<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    pub async fn execute(
        &self,
        user_ids: Vec<UserId>,
    ) -> Result<Vec<(UserId, StoredActivity)>, ActivityError> {
        if user_ids.len() > MAX_BATCH {
            return Err(ActivityError::Invalid);
        }
        Ok(self.store.get_many(&user_ids).await?)
    }
}
