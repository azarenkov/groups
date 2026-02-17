use super::common::{Participant, Room};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantEventRequest {
    pub created_at: u64,
    pub event: String,
    pub id: String,
    pub participant: Participant,
    pub room: Room,
}
