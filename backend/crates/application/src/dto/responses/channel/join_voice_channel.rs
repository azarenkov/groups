use domain::channel::token::ChannelToken;

#[derive(Debug)]
pub struct JoinVoiceChannelResponse {
    pub token: ChannelToken,
    pub url: String,
}

impl JoinVoiceChannelResponse {
    pub fn new(token: ChannelToken, url: String) -> Self {
        Self { token, url }
    }
}
