use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EditMessageRequestBody {
    #[validate(length(min = 1, max = 4000))]
    pub content: String,
}
