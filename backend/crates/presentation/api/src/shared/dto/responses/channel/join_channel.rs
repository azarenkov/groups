use domain::channel::token::ChannelToken;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinChannelResponse {
    pub token: String,
    pub url: String,
}

impl JoinChannelResponse {
    pub fn new(token: ChannelToken, url: String) -> Self {
        Self {
            token: token.as_str().to_owned(),
            url,
        }
    }
}

impl From<application::dto::responses::channel::join_channel::JoinChannelResponse> for JoinChannelResponse {
    fn from(value: application::dto::responses::channel::join_channel::JoinChannelResponse) -> Self {
        Self {
            token: value.token.to_string(),
            url: value.url,
        }
    }
}
