use application::dto::{
    RequestContext, requests::server::create_server::CreateServerRequest,
};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateServerRequestBody {
    #[validate(length(min = 3, max = 20))]
    pub name: String,
    #[validate(length(min = 1, max = 300))]
    pub description: Option<String>,
}

impl CreateServerRequestBody {
    pub fn into_request(self, ctx: RequestContext) -> CreateServerRequest {
        CreateServerRequest {
            name: self.name,
            description: self.description,
            ctx,
        }
    }
}
