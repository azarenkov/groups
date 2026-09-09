use messaging_domain::channel::token::ChannelToken;

#[derive(Debug)]
pub struct JoinVoiceChannelResponse {
    pub token: ChannelToken,
    pub url: String,
    pub muted: bool,
    pub muted_by: Option<uuid::Uuid>,
    pub self_muted: bool,
    pub deafened_by: Option<uuid::Uuid>,
}

impl JoinVoiceChannelResponse {
    pub fn new(
        token: ChannelToken,
        url: String,
        muted: bool,
        muted_by: Option<uuid::Uuid>,
        self_muted: bool,
        deafened_by: Option<uuid::Uuid>,
    ) -> Self {
        Self {
            token,
            url,
            muted,
            muted_by,
            self_muted,
            deafened_by,
        }
    }
}
