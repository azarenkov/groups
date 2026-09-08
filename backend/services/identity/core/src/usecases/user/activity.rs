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
use crate::usecases::user::game_catalog::game_name;

pub const ACTIVITY_TTL: Duration = Duration::from_secs(60);
const OWNER_ABANDONED_AFTER_SECS: i64 = 45;
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
    #[error("Activity is published by another session")]
    Leased,
    #[error("Activity changed concurrently")]
    Superseded,
    #[error("User not found")]
    UserNotFound,
}

pub struct ActivityInput {
    pub game_id: String,
    pub started_at: DateTime<Utc>,
    pub source: String,
}

fn valid_game_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn activity_event(
    user_id: UserId,
    activity: Option<&StoredActivity>,
    occurred_at: DateTime<Utc>,
) -> IdentityEvent {
    IdentityEvent::from(UserEvent::ActivityChanged(UserActivityChanged {
        id: EventId::generate(),
        user_id,
        game_id: activity.map(|a| a.game_id.clone()),
        name: activity.map(|a| a.name.clone()),
        started_at: activity.map(|a| a.started_at),
        occurred_at,
    }))
}

pub struct ActivitySnapshot {
    pub as_of: DateTime<Utc>,
    pub activities: Vec<(UserId, StoredActivity)>,
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
            || !valid_game_id(&input.source)
            || input.started_at > Utc::now() + chrono::Duration::minutes(5)
        {
            return Err(ActivityError::Invalid);
        }
        let name = game_name(&input.game_id).ok_or(ActivityError::Invalid)?;
        let mut tx = self.unit_of_work.begin().await?;
        let user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(ActivityError::UserNotFound)?;
        if user.is_disabled() || user.is_deleted() || !user.show_game_activity() {
            self.store.clear(&user_id).await?;
            tx.save_events(&[activity_event(user_id, None, Utc::now())], None)
                .await?;
            tx.commit().await?;
            return Err(ActivityError::Hidden);
        }
        let previous = self.store.get(&user_id).await?;
        if let Some(prev) = previous.as_ref() {
            if !prev.source.is_empty() && prev.source != input.source {
                let owner_abandoned = Utc::now() - prev.refreshed_at
                    > chrono::Duration::seconds(OWNER_ABANDONED_AFTER_SECS);
                if !owner_abandoned {
                    tx.commit().await?;
                    return Err(ActivityError::Leased);
                }
                if prev.game_id == input.game_id {
                    let adopted = StoredActivity {
                        source: input.source.clone(),
                        refreshed_at: Utc::now(),
                        ..prev.clone()
                    };
                    self.store.set(&user_id, &adopted, ACTIVITY_TTL).await?;
                    tx.commit().await?;
                    return Ok(adopted);
                }
            }
        }
        let needs_publish = previous
            .as_ref()
            .map(|p| p.game_id != input.game_id || !p.published)
            .unwrap_or(true);
        let now = Utc::now();
        let mut activity = StoredActivity {
            game_id: input.game_id,
            name: name.to_string(),
            started_at: input.started_at,
            published: !needs_publish,
            source: input.source,
            refreshed_at: now,
            changed_at: if needs_publish {
                now
            } else {
                previous.as_ref().map(|p| p.changed_at).unwrap_or(now)
            },
        };
        self.store.set(&user_id, &activity, ACTIVITY_TTL).await?;
        if needs_publish {
            tx.save_events(&[activity_event(user_id, Some(&activity), now)], None)
                .await?;
        }
        tx.commit().await?;
        if needs_publish {
            let provisional = activity.clone();
            activity.published = true;
            let finalized = self
                .store
                .set_if_current(&user_id, &provisional, &activity, ACTIVITY_TTL)
                .await?;
            if !finalized {
                return match self.store.get(&user_id).await? {
                    Some(current) if current.source == activity.source && current.published => {
                        Ok(current)
                    }
                    Some(current)
                        if !current.source.is_empty() && current.source != activity.source =>
                    {
                        Err(ActivityError::Leased)
                    }
                    _ => Err(ActivityError::Superseded),
                };
            }
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

    pub async fn execute(
        &self,
        user_id: UserId,
        source: &str,
    ) -> Result<DateTime<Utc>, ActivityError> {
        let mut tx = self.unit_of_work.begin().await?;
        if tx.load_user_for_update(&user_id).await?.is_none() {
            return Err(ActivityError::UserNotFound);
        }
        if let Some(prev) = self.store.get(&user_id).await? {
            if !source.is_empty() && !prev.source.is_empty() && prev.source != source {
                let owner_abandoned = Utc::now() - prev.refreshed_at
                    > chrono::Duration::seconds(OWNER_ABANDONED_AFTER_SECS);
                if !owner_abandoned {
                    tx.commit().await?;
                    return Err(ActivityError::Leased);
                }
            }
        }
        self.store.clear(&user_id).await?;
        let occurred_at = Utc::now();
        tx.save_events(&[activity_event(user_id, None, occurred_at)], None)
            .await?;
        tx.commit().await?;
        Ok(occurred_at)
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
    ) -> Result<ActivitySnapshot, ActivityError> {
        if user_ids.len() > MAX_BATCH {
            return Err(ActivityError::Invalid);
        }
        let visible = self.audience.visible_to(&viewer, &user_ids).await?;
        let as_of = Utc::now();
        if visible.is_empty() {
            return Ok(ActivitySnapshot {
                as_of,
                activities: Vec::new(),
            });
        }
        let activities = self
            .store
            .get_many(&visible)
            .await?
            .into_iter()
            .filter(|(_, activity)| activity.published)
            .collect();
        Ok(ActivitySnapshot { as_of, activities })
    }
}
