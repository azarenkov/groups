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
    pub bio: String,
    pub profile_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<ChannelParticipantDto> for ChannelParticipantResponse {
    fn from(dto: ChannelParticipantDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            bio: dto.bio,
            profile_updated_at: dto.profile_updated_at,
        }
    }
}
