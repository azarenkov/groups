use std::sync::Arc;

use chrono::Utc;
use domain::{event::EventId, user::id::UserId};
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError,
    user::events::UserEvent, user::events::lifecycle::UserActivityChanged,
};
use ports::repository::RepositoryError;

use crate::ports::activity_store::{ActivityStore, ActivityStoreError};
use crate::ports::text_moderation::{TextModeration, TextModerationError};

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
    #[error("Bio is not allowed")]
    BioNotAllowed,
    #[error("Bio moderation unavailable: {0}")]
    ModerationUnavailable(String),
}

pub struct ProfilePatch {
    pub bio: Option<String>,
    pub show_activity: Option<bool>,
    pub show_game_activity: Option<bool>,
    pub timezone: Option<String>,
    pub request_id: Option<String>,
}

pub struct UpdateProfile<W: IdentityUnitOfWork, S: ActivityStore, TM: TextModeration> {
    unit_of_work: Arc<W>,
    activity_store: Arc<S>,
    text_moderation: Arc<TM>,
}

impl<W: IdentityUnitOfWork, S: ActivityStore, TM: TextModeration> UpdateProfile<W, S, TM> {
    pub fn new(unit_of_work: Arc<W>, activity_store: Arc<S>, text_moderation: Arc<TM>) -> Self {
        Self {
            unit_of_work,
            activity_store,
            text_moderation,
        }
    }

    pub async fn execute(&self, user_id: UserId, patch: ProfilePatch) -> Result<(String, bool, bool, chrono::DateTime<Utc>), UpdateProfileError> {
        if let Some(bio) = patch.bio.as_deref().map(str::trim).filter(|b| !b.is_empty()) {
            match self.text_moderation.ensure_allowed(bio).await {
                Ok(()) => {}
                Err(TextModerationError::Rejected) => return Err(UpdateProfileError::BioNotAllowed),
                Err(TextModerationError::Unavailable(e)) => {
                    return Err(UpdateProfileError::ModerationUnavailable(e));
                }
            }
        }
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
        let hidden_at = Utc::now();
        if hid_game_activity {
            events.push(IdentityEvent::from(UserEvent::ActivityChanged(UserActivityChanged {
                id: EventId::generate(),
                user_id,
                game_id: None,
                name: None,
                started_at: None,
                occurred_at: hidden_at,
            })));
        }
        if !events.is_empty() {
            tx.save_user(&user).await?;
            tx.save_events(&events, patch.request_id.as_deref()).await?;
        }
        tx.commit().await?;
        if hid_game_activity
            && let Err(e) = self.activity_store.clear_if_not_newer(&user_id, hidden_at).await
        {
            tracing::warn!(error = %e, user_id = %user_id.uuid(), "Failed to clear the activity cache after hiding game activity");
        }
        Ok((user.bio().to_string(), user.show_activity(), user.show_game_activity(), user.updated_at()))
    }
}
