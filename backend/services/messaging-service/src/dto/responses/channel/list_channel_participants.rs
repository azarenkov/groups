use messaging::dto::responses::channel::channel_participant::ChannelParticipantDto;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListChannelParticipantsResponse {
    pub participants: Vec<ChannelParticipantResponse>,
}

impl ListChannelParticipantsResponse {
    pub fn new(participants: Vec<ChannelParticipantDto>) -> Self {
        Self {
            participants: participants
                .into_iter()
                .map(ChannelParticipantResponse::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelParticipantResponse {
    pub id: Uuid,
    pub name: String,
}

impl From<ChannelParticipantDto> for ChannelParticipantResponse {
    fn from(dto: ChannelParticipantDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
        }
    }
}
