use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JoinVoiceChannelRequestBody {
    pub password: Option<String>,
}
