use async_trait::async_trait;
use contracts::internal::users::{ResolveUsersRequestV1, ResolvedUserV1};
use uuid::Uuid;

use messaging::ports::mention_resolver::MentionResolver;

pub struct IdentityUsersClient {
    client: reqwest::Client,
    base_url: String,
}

impl IdentityUsersClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl MentionResolver for IdentityUsersClient {
    async fn resolve_names(
        &self,
        lowercase_names: &[String],
    ) -> Result<Vec<(Uuid, String)>, String> {
        if lowercase_names.is_empty() {
            return Ok(Vec::new());
        }
        let response = super::with_request_id(
            self.client
                .post(format!("{}/internal/users/resolve", self.base_url))
                .json(&ResolveUsersRequestV1 {
                    names: lowercase_names.to_vec(),
                }),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!("identity resolve failed: {}", response.status()));
        }
        let items: Vec<ResolvedUserV1> =
            response.json().await.map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .map(|u| (u.id, u.name.to_lowercase()))
            .collect())
    }
}

impl IdentityUsersClient {
    pub async fn set_activity_visibility(
        &self,
        user_id: Uuid,
        hidden: bool,
    ) -> Result<(), String> {
        let response = super::with_request_id(
            self.client
                .post(format!(
                    "{}/internal/users/activity-visibility",
                    self.base_url
                ))
                .json(&serde_json::json!({ "userId": user_id, "hidden": hidden })),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err(format!(
                "identity activity visibility failed: {}",
                response.status()
            ));
        }
        Ok(())
    }
}
