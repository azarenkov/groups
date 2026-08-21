use domain::channel::token::ChannelToken;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinVoiceChannelResponse {
    pub token: String,
    pub url: String,
}

impl JoinVoiceChannelResponse {
    pub fn new(token: ChannelToken, url: String) -> Self {
        Self {
            token: token.as_str().to_owned(),
            url,
        }
    }
}

impl From<messaging::dto::responses::channel::join_voice_channel::JoinVoiceChannelResponse>
    for JoinVoiceChannelResponse
{
    fn from(
        value: messaging::dto::responses::channel::join_voice_channel::JoinVoiceChannelResponse,
    ) -> Self {
        Self {
            token: value.token.to_string(),
            url: value.url,
        }
    }
}
