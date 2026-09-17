use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{
    events::IdentityEvent,
    unit_of_work::IdentityUnitOfWork,
    user::UserError,
    user::tier::{BadgeStyle, EntitlementsRepository},
};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum SetBadgeStyleError {
    #[error("Unknown badge style")]
    UnknownStyle,
    #[error("A paid plan is required")]
    ProRequired,
    #[error("User not found")]
    UserNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl From<UserError> for SetBadgeStyleError {
    fn from(_: UserError) -> Self {
        Self::UserNotFound
    }
}

pub struct SetBadgeStyle<W: IdentityUnitOfWork, E: EntitlementsRepository> {
    unit_of_work: Arc<W>,
    entitlements: Arc<E>,
}

impl<W: IdentityUnitOfWork, E: EntitlementsRepository> SetBadgeStyle<W, E> {
    pub fn new(unit_of_work: Arc<W>, entitlements: Arc<E>) -> Self {
        Self {
            unit_of_work,
            entitlements,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        style: &str,
        request_id: Option<&str>,
    ) -> Result<BadgeStyle, SetBadgeStyleError> {
        let style = BadgeStyle::parse(style).ok_or(SetBadgeStyleError::UnknownStyle)?;
        let now = Utc::now();
        if style.needs_paid_tier()
            && !self.entitlements.get(&user_id).await?.active_at(now).is_paid()
        {
            return Err(SetBadgeStyleError::ProRequired);
        }
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(SetBadgeStyleError::UserNotFound)?;
        user.set_badge_style(style, now)?;
        let events: Vec<IdentityEvent> =
            user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_user(&user).await?;
        tx.save_events(&events, request_id).await?;
        tx.commit().await?;
        Ok(style)
    }
}
