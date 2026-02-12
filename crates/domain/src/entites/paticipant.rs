use chrono::{DateTime, Utc};

use crate::value_objects::paticipant::{
    identity::ParticipantIdentity, name::ParticipantName, sid::ParticipantSid,
    state::ParticipantState,
};

#[derive(Debug)]
pub struct Participant {
    sid: ParticipantSid,
    identity: ParticipantIdentity,
    name: ParticipantName,
    state: ParticipantState,
    joined_at: DateTime<Utc>,
    metadata: Option<String>,
}

impl Participant {
    pub fn new(
        sid: ParticipantSid,
        identity: ParticipantIdentity,
        name: ParticipantName,
        state: ParticipantState,
        joined_at: DateTime<Utc>,
        metadata: Option<String>,
    ) -> Self {
        Self {
            sid,
            identity,
            name,
            state,
            joined_at,
            metadata,
        }
    }
}
