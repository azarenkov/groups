use api_common::internal_client::{InternalClient, expect_success};
use uuid::Uuid;

pub struct IdentityUsersClient {
    identity: InternalClient,
}

impl IdentityUsersClient {
    pub fn new(base_url: &str) -> Self {
        Self { identity: InternalClient::new(base_url) }
    }
}

impl IdentityUsersClient {
    pub async fn set_activity_visibility(&self, user_id: Uuid, hidden: bool) -> Result<(), String> {
        let request = self
            .identity
            .post("/internal/users/activity-visibility")
            .json(&serde_json::json!({ "userId": user_id, "hidden": hidden }));
        expect_success(request, "identity activity visibility").await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
