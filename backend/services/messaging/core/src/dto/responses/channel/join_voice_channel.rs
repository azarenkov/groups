use domain::channel::token::ChannelToken;

#[derive(Debug)]
pub struct JoinVoiceChannelResponse {
    pub token: ChannelToken,
    pub url: String,
    pub muted: bool,
}

impl JoinVoiceChannelResponse {
    pub fn new(token: ChannelToken, url: String, muted: bool) -> Self {
        Self { token, url, muted }
    }
}
