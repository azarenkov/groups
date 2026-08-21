use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EditMessageRequestBody {
    #[validate(length(max = 2000))]
    pub content: String,
    #[serde(default)]
    pub add_attachment_ids: Vec<Uuid>,
    #[serde(default)]
    pub remove_attachment_ids: Vec<Uuid>,
}
