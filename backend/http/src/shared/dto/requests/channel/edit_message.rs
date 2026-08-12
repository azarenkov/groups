use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EditMessageRequestBody {
    #[validate(length(max = 2000))]
    pub content: String,
}
