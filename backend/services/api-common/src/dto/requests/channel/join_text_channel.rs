use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JoinTextChannelRequestBody {
    pub password: Option<String>,
}
