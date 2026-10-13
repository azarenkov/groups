use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[async_trait]
pub trait BannerRepository: Send + Sync {
    async fn get_banner_key(&self, user_id: &UserId) -> Result<Option<String>, RepositoryError>;
    async fn set_banner_key(
        &self,
        user_id: &UserId,
        key: Option<&str>,
    ) -> Result<Option<String>, RepositoryError>;
}
