use api_common::internal_client::{InternalClient, expect_json};
use async_trait::async_trait;
use contracts::internal::calls::{CallOutcomeResponseV1, CallRoomsRequestV1, CallRoomsResponseV1, EndedCallResponseV1};
use domain::user::id::UserId;
use messaging::ports::call_participation::{
    CallInvitation, CallOutcome, CallParticipationError, CallParticipationLookup, EndedCall,
};
use uuid::Uuid;

pub struct VoiceCallsClient {
    voice: InternalClient,
}

impl VoiceCallsClient {
    pub fn new(base_url: &str) -> Self {
        Self { voice: InternalClient::new(base_url) }
    }
}

#[async_trait]
impl CallParticipationLookup for VoiceCallsClient {
    async fn call_outcome(&self, call_id: Uuid, caller: &UserId, callee: &UserId) -> Result<CallOutcome, CallParticipationError> {
        let request = self
            .voice
            .get(&format!("/internal/calls/{}/outcome/{}/{}", call_id, caller.uuid(), callee.uuid()));
        let body: CallOutcomeResponseV1 = expect_json(request, "call outcome")
            .await
            .map_err(|e| CallParticipationError::Unavailable(e.to_string()))?;
        Ok(CallOutcome {
            participated: body.participated,
            caller_in_call: body.caller_in_call,
            caller_left_at: body.caller_left_at,
            callee_left_at: body.callee_left_at,
            callee_invited_by_caller: body.callee_invited_by_caller,
            callee_pending: body.callee_pending,
            callee_answered_at: body.callee_answered_at,
            callee_declined: body.callee_declined,
        })
    }

    async fn ended_call(&self, call_id: Uuid) -> Result<Option<EndedCall>, CallParticipationError> {
        let request = self.voice.get(&format!("/internal/calls/{call_id}/ended"));
        let body: Option<EndedCallResponseV1> = expect_json(request, "ended call")
            .await
            .map_err(|e| CallParticipationError::Unavailable(e.to_string()))?;
        Ok(body.map(|call| EndedCall {
            ended_at: call.ended_at,
            invitations: call
                .invitations
                .into_iter()
                .map(|i| CallInvitation { inviter: UserId::new(i.inviter_id), invitee: UserId::new(i.invitee_id) })
                .collect(),
        }))
    }

    async fn call_rooms_among(&self, ids: &[Uuid]) -> Result<Vec<Uuid>, CallParticipationError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let request = self.voice.post("/internal/calls/rooms").json(&CallRoomsRequestV1 { ids: ids.to_vec() });
        let body: CallRoomsResponseV1 = expect_json(request, "call rooms")
            .await
            .map_err(|e| CallParticipationError::Unavailable(e.to_string()))?;
        Ok(body.call_ids)
    }
}
