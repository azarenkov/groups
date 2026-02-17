use super::common::{Participant, Room};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackEventRequest {
    pub created_at: String,
    pub id: String,
    pub participant: Participant,
    pub room: Room,
    pub track: Track,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    #[serde(rename = "type")]
    pub r#type: String,
}
