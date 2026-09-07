use std::sync::Arc;

use chrono::Utc;
use domain::{event::EventId, user::id::UserId};
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError,
    user::events::UserEvent, user::events::lifecycle::UserActivityChanged,
};
use ports::repository::RepositoryError;

use crate::ports::activity_store::{ActivityStore, ActivityStoreError};

#[derive(Debug, thiserror::Error)]
pub enum UpdateProfileError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Bio is too long")]
    BioTooLong,
    #[error("User not found")]
    UserNotFound,
    #[error("Activity store error: {0}")]
    ActivityStore(#[from] ActivityStoreError),
}

pub struct ProfilePatch {
    pub bio: Option<String>,
    pub show_activity: Option<bool>,
    pub show_game_activity: Option<bool>,
    pub timezone: Option<String>,
    pub request_id: Option<String>,
}

pub struct UpdateProfile<W: IdentityUnitOfWork, S: ActivityStore> {
    unit_of_work: Arc<W>,
    activity_store: Arc<S>,
}

impl<W: IdentityUnitOfWork, S: ActivityStore> UpdateProfile<W, S> {
    pub fn new(unit_of_work: Arc<W>, activity_store: Arc<S>) -> Self {
        Self {
            unit_of_work,
            activity_store,
        }
    }

    pub async fn execute(&self, user_id: UserId, patch: ProfilePatch) -> Result<(String, bool, bool, chrono::DateTime<Utc>), UpdateProfileError> {
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(UpdateProfileError::UserNotFound)?;
        let bio = patch.bio.unwrap_or_else(|| user.bio().to_string());
        let show_activity = patch.show_activity.unwrap_or(user.show_activity());
        let show_game_activity = patch.show_game_activity.unwrap_or(user.show_game_activity());
        let hid_game_activity = user.show_game_activity() && !show_game_activity;
        let timezone = patch.timezone.unwrap_or_else(|| user.timezone().to_string());
        user.update_profile(bio, show_activity, show_game_activity, timezone, Utc::now()).map_err(|e| match e {
            UserError::BioTooLong => UpdateProfileError::BioTooLong,
            _ => UpdateProfileError::UserNotFound,
        })?;
        let mut events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        if !show_game_activity {
            let previous = self.activity_store.clear(&user_id).await?;
            if hid_game_activity || previous.is_some() {
                events.push(IdentityEvent::from(UserEvent::ActivityChanged(UserActivityChanged {
                    id: EventId::generate(),
                    user_id,
                    game_id: None,
                    name: None,
                    started_at: None,
                    occurred_at: Utc::now(),
                })));
            }
        }
        if !events.is_empty() {
            tx.save_user(&user).await?;
            tx.save_events(&events, patch.request_id.as_deref()).await?;
        }
        tx.commit().await?;
        Ok((user.bio().to_string(), user.show_activity(), user.show_game_activity(), user.updated_at()))
    }
}
