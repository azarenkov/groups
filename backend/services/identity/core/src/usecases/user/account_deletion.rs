use chrono::{DateTime, Utc};
use domain::event::EventId;
use domain::user::id::UserId;
use identity_domain::{
    events::IdentityEvent, unit_of_work::IdentityTransaction, unit_of_work::IdentityUnitOfWork, user::User,
    user::access_revocation::AccessRevocation,
    user::events::UserEvent, user::events::lifecycle::UserActivityChanged,
};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        access_revoker::AccessRevoker,
        activity_store::{ActivityStore, ActivityStoreError},
    },
    usecases::auth::reactivation::revoke_access_after_commit,
};

#[derive(Debug, thiserror::Error)]
pub enum AccountDeletionError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
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
    tx.record_access_revocation(&AccessRevocation::new(user_id, now)).await?;
    tx.save_events(&events, request_id).await?;
    Ok(())
}

pub async fn clear_deleted_user_caches<W, S, R>(
    unit_of_work: &W,
    activity_store: &S,
    access_revoker: &R,
    user_id: &UserId,
    now: DateTime<Utc>,
) -> Result<(), AccountDeletionError>
where
    W: IdentityUnitOfWork + ?Sized,
    S: ActivityStore + ?Sized,
    R: AccessRevoker + ?Sized,
{
    let activity = activity_store.clear(user_id).await;
    revoke_access_after_commit(unit_of_work, access_revoker, &AccessRevocation::new(*user_id, now)).await;
    activity?;
    Ok(())
}
