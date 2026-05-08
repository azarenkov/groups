use serde::Deserialize;

use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateServerRequest {
    #[validate(length(min = 3, max = 20))]
    pub name: String,
    #[validate(length(min = 1, max = 300))]
    pub description: Option<String>,
}

impl From<CreateServerRequest>
    for application::dto::requests::server::create_server::CreateServerRequest
{
    fn from(value: CreateServerRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
        }
    }
}
