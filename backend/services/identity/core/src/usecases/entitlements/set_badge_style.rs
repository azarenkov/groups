use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{
    events::IdentityEvent,
    unit_of_work::IdentityUnitOfWork,
    user::UserError,
    user::tier::{BadgeStyle, Tier},
};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum SetBadgeStyleError {
    #[error("Unknown badge style")]
    UnknownStyle,
    #[error("A paid plan is required")]
    ProRequired,
    #[error("Premium is required")]
    PremiumRequired,
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

pub struct SetBadgeStyle<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> SetBadgeStyle<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        style: &str,
        request_id: Option<&str>,
    ) -> Result<BadgeStyle, SetBadgeStyleError> {
        let style = BadgeStyle::parse(style).ok_or(SetBadgeStyleError::UnknownStyle)?;
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(SetBadgeStyleError::UserNotFound)?;
        let now = Utc::now();
        let tier = tx.load_entitlements(&user_id).await?.active_at(now);
        if !tier.includes(style.required_tier()) {
            return Err(if style.required_tier() == Tier::Premium {
                SetBadgeStyleError::PremiumRequired
            } else {
                SetBadgeStyleError::ProRequired
            });
        }
        user.set_badge_style(style, now)?;
        let events: Vec<IdentityEvent> =
            user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_user(&user).await?;
        tx.save_events(&events, request_id).await?;
        tx.commit().await?;
        Ok(style)
    }
}
