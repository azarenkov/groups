use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::ports::hasher::{HashError, Hasher};

#[derive(Debug, thiserror::Error)]
pub enum DisableAccountError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Password is incorrect")]
    InvalidPassword,
    #[error("User not found")]
    UserNotFound,
}

pub struct DisableAccount<U, W, H>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
}

impl<U, W, H> DisableAccount<U, W, H>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
{
    pub fn new(user_repository: Arc<U>, unit_of_work: Arc<W>, hasher: Arc<H>) -> Self {
        Self {
            user_repository,
            unit_of_work,
            hasher,
        }
    }

    pub async fn execute(&self, user_id: UserId, password: &str, request_id: Option<&str>) -> Result<(), DisableAccountError> {
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(DisableAccountError::UserNotFound)?;
        if user.is_deleted() {
            return Err(DisableAccountError::UserNotFound);
        }
        if !self.hasher.verify(password, user.password_hash().as_str())? {
            return Err(DisableAccountError::InvalidPassword);
        }

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(DisableAccountError::UserNotFound)?;
        user.disable(now).map_err(|_| DisableAccountError::UserNotFound)?;
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_user(&user).await?;
        tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
        tx.save_events(&events, request_id).await?;
        tx.commit().await?;
        Ok(())
    }
}
