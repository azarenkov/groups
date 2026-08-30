use async_trait::async_trait;
use chrono::{DateTime, Utc};

use domain::user::id::UserId;

use crate::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct PublicProfile {
    pub user_id: UserId,
    pub name: String,
    pub bio: String,
}

#[async_trait]
pub trait UserProfileRepository {
    async fn get_public_profile(&self, user_id: &UserId) -> Result<Option<PublicProfile>, RepositoryError>;
    async fn update_profile(
        &self,
        user_id: &UserId,
        bio: &str,
        show_activity: bool,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn get_name(&self, user_id: &UserId) -> Result<Option<String>, RepositoryError>;
    async fn find_user_by_name(&self, name: &str) -> Result<Option<UserId>, RepositoryError>;
    async fn upsert_name(
        &self,
        user_id: &UserId,
        name: &str,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn is_active(&self, user_id: &UserId) -> Result<bool, RepositoryError>;
    async fn is_deactivation_current(
        &self,
        user_id: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;
    async fn set_active(
        &self,
        user_id: &UserId,
        active: bool,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;
}
