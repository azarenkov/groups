use std::sync::Arc;

use chrono::{Duration, Utc};
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::events::UserEvent,
    user::events::lifecycle::UserTrialEnding, user::repository::UserRepository,
};
use ports::repository::RepositoryError;

pub const TRIAL_REMINDER_LEAD_HOURS: i64 = 24;

pub struct RemindTrialEnding<W: IdentityUnitOfWork, U: UserRepository> {
    unit_of_work: Arc<W>,
    users: Arc<U>,
    batch_size: i64,
}

impl<W: IdentityUnitOfWork, U: UserRepository> RemindTrialEnding<W, U> {
    pub fn new(unit_of_work: Arc<W>, users: Arc<U>, batch_size: i64) -> Self {
        Self {
            unit_of_work,
            users,
            batch_size,
        }
    }

    pub async fn drain(&self) -> Result<usize, RepositoryError> {
        let mut total = 0;
        loop {
            let reminded = self.run_once().await?;
            total += reminded;
            if (reminded as i64) < self.batch_size {
                return Ok(total);
            }
        }
    }

    pub async fn run_once(&self) -> Result<usize, RepositoryError> {
        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let ending = tx
            .load_trials_ending_for_update(now + Duration::hours(TRIAL_REMINDER_LEAD_HOURS), self.batch_size)
            .await?;
        let mut events = Vec::with_capacity(ending.len());
        for user_id in &ending {
            tx.mark_trial_reminded(user_id, now).await?;
            let Some(user) = self.users.find_by_id(user_id).await? else {
                continue;
            };
            if user.is_deleted() {
                continue;
            }
            let Some(ends_at) = tx.load_entitlements(user_id).await?.valid_until else {
                continue;
            };
            events.push(IdentityEvent::from(UserEvent::TrialEnding(UserTrialEnding::record(
                *user_id,
                user.email().clone(),
                user.locale().as_str().to_string(),
                ends_at,
                now,
            ))));
        }
        if !events.is_empty() {
            tx.save_events(&events, None).await?;
        }
        tx.commit().await?;
        Ok(ending.len())
    }
}
