use domain::channel::token::ChannelToken;

#[derive(Debug)]
pub struct JoinChannelResponse {
    pub token: ChannelToken,
    pub url: String,
}

impl JoinChannelResponse {
    pub fn new(token: ChannelToken, url: String) -> Self {
        Self { token, url }
    }
}
