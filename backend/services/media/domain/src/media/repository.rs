use async_trait::async_trait;
use chrono::{DateTime, Utc};

use domain::{community::id::CommunityId, user::id::UserId};
use ports::repository::RepositoryError;

#[async_trait]
pub trait MediaRepository {
    async fn get_user_avatar_key(
        &self,
        user_id: &UserId,
    ) -> Result<Option<String>, RepositoryError>;
    async fn set_user_avatar_key(
        &self,
        user_id: &UserId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError>;
    async fn mark_user_deleted(
        &self,
        user_id: &UserId,
        deleted_at: DateTime<Utc>,
    ) -> Result<Option<String>, RepositoryError>;
    async fn get_community_logo_key(
        &self,
        community_id: &CommunityId,
    ) -> Result<Option<String>, RepositoryError>;
    async fn set_community_logo_key(
        &self,
        community_id: &CommunityId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError>;
    async fn get_community_owner(
        &self,
        community_id: &CommunityId,
    ) -> Result<Option<UserId>, RepositoryError>;
    async fn upsert_community_owner(
        &self,
        community_id: &CommunityId,
        owner_id: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn remove_community_media(
        &self,
        community_id: &CommunityId,
        owner_id: &UserId,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
}
