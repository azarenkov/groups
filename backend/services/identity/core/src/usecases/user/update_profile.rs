use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::UserError};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum UpdateProfileError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Bio is too long")]
    BioTooLong,
    #[error("User not found")]
    UserNotFound,
}

pub struct ProfilePatch {
    pub bio: Option<String>,
    pub show_activity: Option<bool>,
    pub request_id: Option<String>,
}

pub struct UpdateProfile<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> UpdateProfile<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(&self, user_id: UserId, patch: ProfilePatch) -> Result<(String, bool), UpdateProfileError> {
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(UpdateProfileError::UserNotFound)?;
        let bio = patch.bio.unwrap_or_else(|| user.bio().to_string());
        let show_activity = patch.show_activity.unwrap_or(user.show_activity());
        user.update_profile(bio, show_activity, Utc::now()).map_err(|e| match e {
            UserError::BioTooLong => UpdateProfileError::BioTooLong,
            _ => UpdateProfileError::UserNotFound,
        })?;
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        if !events.is_empty() {
            tx.save_user(&user).await?;
            tx.save_events(&events, patch.request_id.as_deref()).await?;
        }
        tx.commit().await?;
        Ok((user.bio().to_string(), user.show_activity()))
    }
}
