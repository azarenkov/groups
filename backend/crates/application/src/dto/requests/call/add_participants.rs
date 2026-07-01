use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct AddCallParticipantsRequest {
    pub call_id: Uuid,
    pub invitee_ids: Vec<Uuid>,
    pub ctx: RequestContext,
}
