use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait MentionResolver {
    async fn resolve_names(
        &self,
        lowercase_names: &[String],
    ) -> Result<Vec<(Uuid, String)>, String>;
}
