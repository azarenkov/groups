use async_trait::async_trait;
use contracts::internal::calls::{CallOutcomeResponseV1, EndedCallResponseV1};
use domain::user::id::UserId;
use messaging::ports::call_participation::{
    CallInvitation, CallOutcome, CallParticipationError, CallParticipationLookup, EndedCall,
};
use uuid::Uuid;

pub struct VoiceCallsClient {
    client: reqwest::Client,
    base_url: String,
}

impl VoiceCallsClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

#[async_trait]
impl CallParticipationLookup for VoiceCallsClient {
    async fn call_outcome(
        &self,
        call_id: Uuid,
        caller: &UserId,
        callee: &UserId,
    ) -> Result<CallOutcome, CallParticipationError> {
        let response = super::with_request_id(self.client.get(format!(
            "{}/internal/calls/{}/outcome/{}/{}",
            self.base_url,
            call_id,
            caller.uuid(),
            callee.uuid()
        )))
        .send()
        .await
        .map_err(|e| CallParticipationError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(CallParticipationError::Unavailable(format!(
                "call outcome endpoint returned status {}",
                response.status()
            )));
        }
        let body: CallOutcomeResponseV1 = response
            .json()
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
        let response = super::with_request_id(
            self.client.get(format!("{}/internal/calls/{}/ended", self.base_url, call_id)),
        )
        .send()
        .await
        .map_err(|e| CallParticipationError::Unavailable(e.to_string()))?;
        if !response.status().is_success() {
            return Err(CallParticipationError::Unavailable(format!(
                "ended call endpoint returned status {}",
                response.status()
            )));
        }
        let body: Option<EndedCallResponseV1> = response
            .json()
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
}
