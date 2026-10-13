use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinVoiceChannelResponse {
    pub token: String,
    pub url: String,
    pub muted: bool,
    pub muted_by: Option<uuid::Uuid>,
    pub self_muted: bool,
    pub deafened_by: Option<uuid::Uuid>,
    pub stream_ceiling: domain::stream::StreamCeiling,
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
            muted: value.muted,
            muted_by: value.muted_by,
            self_muted: value.self_muted,
            deafened_by: value.deafened_by,
            stream_ceiling: value.stream_ceiling,
        }
    }
}
