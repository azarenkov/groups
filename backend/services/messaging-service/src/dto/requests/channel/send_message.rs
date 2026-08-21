use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequestBody {
    #[validate(length(max = 2000))]
    pub content: String,
    #[serde(default)]
    pub attachment_ids: Vec<Uuid>,
}
