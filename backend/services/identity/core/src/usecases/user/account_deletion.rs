use chrono::{DateTime, Utc};
use domain::event::EventId;
use domain::user::id::UserId;
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityTransaction, user::User,
    user::events::UserEvent, user::events::lifecycle::UserActivityChanged,
};
use ports::repository::RepositoryError;

use crate::ports::{
    access_revoker::{AccessRevoker, AccessRevokerError},
    activity_store::{ActivityStore, ActivityStoreError},
};

#[derive(Debug, thiserror::Error)]
pub enum AccountDeletionError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Access revocation error: {0}")]
    AccessRevoker(#[from] AccessRevokerError),
    #[error("Activity store error: {0}")]
    ActivityStore(#[from] ActivityStoreError),
    #[error("Account is already deleted")]
    AlreadyDeleted,
}

pub async fn apply_account_deletion(
    tx: &mut dyn IdentityTransaction,
    user: &mut User,
    now: DateTime<Utc>,
    request_id: Option<&str>,
) -> Result<(), AccountDeletionError> {
    let user_id = user.id_ownned();
    user.delete(now)
        .map_err(|_| AccountDeletionError::AlreadyDeleted)?;
    let mut events: Vec<IdentityEvent> =
        user.pull_events().into_iter().map(IdentityEvent::from).collect();
    events.push(IdentityEvent::from(UserEvent::ActivityChanged(
        UserActivityChanged {
            id: EventId::generate(),
            user_id,
            game_id: None,
            name: None,
            started_at: None,
            occurred_at: now,
        },
    )));
    tx.save_user(user).await?;
    tx.delete_oauth_identities(&user_id).await?;
    tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
    tx.save_events(&events, request_id).await?;
    Ok(())
}

pub async fn clear_deleted_user_caches<S, R>(
    activity_store: &S,
    access_revoker: &R,
    user_id: &UserId,
    now: DateTime<Utc>,
) -> Result<(), AccountDeletionError>
where
    S: ActivityStore + ?Sized,
    R: AccessRevoker + ?Sized,
{
    let activity = activity_store.clear(user_id).await;
    let access = access_revoker.revoke_access_issued_before(user_id, now).await;
    access?;
    activity?;
    Ok(())
}
