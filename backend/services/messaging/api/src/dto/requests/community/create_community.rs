use messaging::dto::{
    RequestContext, requests::community::create_community::CreateCommunityRequest,
};
use domain::user::role::UserRole;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommunityRequestBody {
    #[validate(length(min = 3, max = 20))]
    pub name: String,
    #[validate(length(min = 1, max = 300))]
    pub description: Option<String>,
    #[validate(length(equal = 7))]
    pub accent_color: Option<String>,
}

impl CreateCommunityRequestBody {
    pub fn into_request(self, ctx: RequestContext, creator_role: UserRole) -> CreateCommunityRequest {
        CreateCommunityRequest {
            name: self.name,
            description: self.description,
            accent_color: self.accent_color,
            creator_role,
            ctx,
        }
    }
}
