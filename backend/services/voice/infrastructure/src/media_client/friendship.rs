use async_trait::async_trait;
use voice::ports::friendship_lookup::{FriendshipLookup, FriendshipLookupError};
use domain::user::id::UserId;
use contracts::internal::friendships::FriendsResponseV1;

pub struct MessagingFriendshipClient {
    client: reqwest::Client,
    base_url: String,
}

impl MessagingFriendshipClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl FriendshipLookup for MessagingFriendshipClient {
    async fn are_friends(&self, a: &UserId, b: &UserId) -> Result<bool, FriendshipLookupError> {
        let response = super::with_request_id(
            self.client
            .get(format!(
                "{}/internal/friendships/{}/{}",
                self.base_url,
                a.uuid(),
                b.uuid()
            )))
            .send()
            .await
            .map_err(|e| FriendshipLookupError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(FriendshipLookupError::Unavailable(format!(
                "friendship endpoint returned status {}",
                response.status()
            )));
        }
        let body: FriendsResponseV1 = response
            .json()
            .await
            .map_err(|e| FriendshipLookupError::Unavailable(e.to_string()))?;
        Ok(body.friends)
    }
}
