use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::unit_of_work::IdentityUnitOfWork;
use identity_domain::user::repository::UserRepository;
use identity_domain::user::settings::appearance::{
    AppearanceError as DomainAppearanceError, AppearancePatch, AppearanceSettings,
};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum AppearanceError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid appearance settings: {0}")]
    Invalid(#[from] DomainAppearanceError),
}

pub struct GetAppearance<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> GetAppearance<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Option<AppearanceSettings>, AppearanceError> {
        Ok(self.user_repository.get_appearance(&user_id).await?)
    }
}

pub struct UpdateAppearance<W: IdentityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: IdentityUnitOfWork> UpdateAppearance<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        patch: AppearancePatch,
    ) -> Result<AppearanceSettings, AppearanceError> {
        let mut tx = self.unit_of_work.begin().await?;
        let current = tx
            .load_appearance_for_update(&user_id)
            .await?
            .unwrap_or_default();
        let next = current.apply(&patch)?;
        tx.save_appearance(&user_id, &next).await?;
        tx.commit().await?;
        Ok(next)
    }
}
