use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use domain::user::id::UserId;
use identity_domain::{
    events::IdentityEvent,
    unit_of_work::IdentityUnitOfWork,
    user::events::UserEvent,
    user::events::lifecycle::UserEntitlementsChanged,
    user::tier::{Entitlements, Tier},
};
use ports::repository::RepositoryError;

use crate::usecases::auth::captcha::normalize_ip;

#[derive(Debug, thiserror::Error)]
pub enum StartTrialError {
    #[error("User not found")]
    UserNotFound,
    #[error("Email is not verified")]
    EmailNotVerified,
    #[error("Trial was already used")]
    AlreadyUsed,
    #[error("Account already has a paid plan")]
    AlreadyPaid,
    #[error("Too many trials from this network")]
    TooManyFromNetwork,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct StartTrial<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
    ip_limit: i64,
    ip_window: Duration,
}

impl<W: IdentityUnitOfWork> StartTrial<W> {
    pub fn new(unit_of_work: Arc<W>, ip_limit: i64, ip_window: Duration) -> Self {
        Self {
            unit_of_work,
            ip_limit,
            ip_window,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        ip: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<DateTime<Utc>, StartTrialError> {
        let now = Utc::now();
        let ip = ip.and_then(normalize_ip);
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .filter(|user| !user.is_deleted())
            .ok_or(StartTrialError::UserNotFound)?;
        if !user.is_email_verified() {
            return Err(StartTrialError::EmailNotVerified);
        }
        let entitlements = tx.load_entitlements(&user_id).await?;
        if entitlements.trial_used_at.is_some() {
            return Err(StartTrialError::AlreadyUsed);
        }
        if entitlements.active_at(now).is_paid() {
            return Err(StartTrialError::AlreadyPaid);
        }
        let history = tx
            .trial_history(user.email(), ip.as_deref(), now - self.ip_window)
            .await?;
        if history.email_used {
            return Err(StartTrialError::AlreadyUsed);
        }
        if history.recent_from_ip >= self.ip_limit {
            return Err(StartTrialError::TooManyFromNetwork);
        }
        let valid_until = Entitlements::trial_ends_at(now);
        tx.start_trial(&user_id, user.email(), ip.as_deref(), valid_until, now)
            .await?;
        let changed_at = Utc::now();
        let mut events = vec![IdentityEvent::from(UserEvent::EntitlementsChanged(
            UserEntitlementsChanged::record(
                user_id,
                Tier::Pro.as_str().to_string(),
                Some(valid_until),
                entitlements.member_number,
                changed_at,
            )
            .trial(),
        ))];
        let lapsed = user.fit_to_tier(entitlements.active_at(now), changed_at);
        if user.fit_to_tier(Tier::Pro, changed_at) || lapsed {
            events.extend(user.pull_events().into_iter().map(IdentityEvent::from));
            tx.save_user(&user).await?;
        }
        tx.save_events(&events, request_id).await?;
        tx.commit().await?;
        Ok(valid_until)
    }
}
