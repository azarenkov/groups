use application::dto::{
    RequestContext, requests::server::update_server::UpdateServerRequest,
};
use domain::{server::id::ServerId, user::id::UserId};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateServerRequestBody {
    #[validate(length(min = 3, max = 20))]
    pub name: String,
    #[validate(length(min = 1, max = 300))]
    pub description: Option<String>,
}

impl UpdateServerRequestBody {
    pub fn into_request(
        self,
        server_id: ServerId,
        owner_id: UserId,
        ctx: RequestContext,
    ) -> UpdateServerRequest {
        UpdateServerRequest {
            server_id,
            owner_id,
            name: self.name,
            description: self.description,
            ctx,
        }
    }
}
