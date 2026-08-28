use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use identity_domain::{unit_of_work::IdentityUnitOfWork, user::locale::Locale};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum UpdateLocaleError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct UpdateUserLocale<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> UpdateUserLocale<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(&self, user_id: UserId, locale: Locale) -> Result<(), UpdateLocaleError> {
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(RepositoryError::NotFound)?;
        user.set_locale(locale, Utc::now());
        tx.save_user(&user).await?;
        tx.commit().await?;
        Ok(())
    }
}
