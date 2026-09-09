use uuid::Uuid;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct StartCallRequest {
    pub invitee_ids: Vec<Uuid>,
    pub video: bool,
    pub ctx: RequestContext,
}
