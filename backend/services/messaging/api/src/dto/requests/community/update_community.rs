use messaging::dto::{
    RequestContext, requests::community::update_community::UpdateCommunityRequest,
};
use messaging_domain::community::id::CommunityId;
use domain::user::id::UserId;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCommunityRequestBody {
    #[validate(length(min = 3, max = 20))]
    pub name: String,
    #[validate(length(min = 1, max = 300))]
    pub description: Option<String>,
}

impl UpdateCommunityRequestBody {
    pub fn into_request(
        self,
        community_id: CommunityId,
        owner_id: UserId,
        ctx: RequestContext,
    ) -> UpdateCommunityRequest {
        UpdateCommunityRequest {
            community_id,
            owner_id,
            name: self.name,
            description: self.description,
            ctx,
        }
    }
}
