use application::ports::voice_webhook_verifier::{
    VerifiedVoiceWebhookEvent, VoiceWebhookEvent, VoiceWebhookVerifier, VoiceWebhookVerifierError,
    WebhookSource,
};
use chrono::{DateTime, Utc};
use domain::channel::events::participant::{ConnectionAbortReason, LeaveReason};
use livekit_api::{access_token::TokenVerifier, webhooks::WebhookReceiver};
use livekit_protocol::{DisconnectReason, ParticipantInfo, Room};
use uuid::Uuid;

use crate::config::livekit::LiveKitConfig;

const EVENT_PARTICIPANT_CONNECTION_ABORTED: &str = "participant_connection_aborted";
const EVENT_PARTICIPANT_LEFT: &str = "participant_left";
const EVENT_PARTICIPANT_JOINED: &str = "participant_joined";

pub struct LiveKitWebhookReceiver {
    receiver: WebhookReceiver,
}

impl LiveKitWebhookReceiver {
    pub fn new(config: &LiveKitConfig) -> Self {
        let token_verifier = TokenVerifier::with_api_key(config.api_key(), config.api_secret());
        Self {
            receiver: WebhookReceiver::new(token_verifier),
        }
    }
}

impl VoiceWebhookVerifier for LiveKitWebhookReceiver {
    fn verify(
        &self,
        body: &str,
        auth_token: &str,
    ) -> Result<VerifiedVoiceWebhookEvent, VoiceWebhookVerifierError> {
        let event = self.receiver.receive(body, auth_token).map_err(map_err)?;

        if event.id.is_empty() {
            return Err(VoiceWebhookVerifierError::MissingField("id"));
        }
        let external_id = event.id.clone();
        let occurred_at =
            DateTime::<Utc>::from_timestamp(event.created_at, 0).unwrap_or_else(Utc::now);

        let voice_event = match event.event.as_str() {
            EVENT_PARTICIPANT_JOINED => {
                let participant = event
                    .participant
                    .as_ref()
                    .ok_or(VoiceWebhookVerifierError::MissingField("participant"))?;
                match extract_ids(event.room.as_ref(), Some(participant))? {
                    Some((channel_id, user_id)) => {
                        let session_id = require_sid(participant)?;
                        VoiceWebhookEvent::ParticipantJoined {
                            channel_id,
                            user_id,
                            session_id,
                        }
                    }
                    None => VoiceWebhookEvent::Other,
                }
            }
            EVENT_PARTICIPANT_CONNECTION_ABORTED => {
                let participant = event
                    .participant
                    .as_ref()
                    .ok_or(VoiceWebhookVerifierError::MissingField("participant"))?;
                match extract_ids(event.room.as_ref(), Some(participant))? {
                    Some((channel_id, user_id)) => {
                        let reason = parse_disconnect_reason(participant.disconnect_reason);
                        let session_id = require_sid(participant)?;
                        VoiceWebhookEvent::ParticipantConnectionAborted {
                            channel_id,
                            user_id,
                            session_id,
                            reason: abort_reason_from(reason),
                        }
                    }
                    None => VoiceWebhookEvent::Other,
                }
            }
            EVENT_PARTICIPANT_LEFT => {
                let participant = event
                    .participant
                    .as_ref()
                    .ok_or(VoiceWebhookVerifierError::MissingField("participant"))?;
                match extract_ids(event.room.as_ref(), Some(participant))? {
                    Some((channel_id, user_id)) => {
                        let reason = parse_disconnect_reason(participant.disconnect_reason);
                        let session_id = require_sid(participant)?;
                        map_leave_reason(reason, channel_id, user_id, session_id)
                    }
                    None => VoiceWebhookEvent::Other,
                }
            }
            _ => VoiceWebhookEvent::Other,
        };

        Ok(VerifiedVoiceWebhookEvent {
            source: WebhookSource::LiveKit,
            external_id,
            occurred_at,
            event: voice_event,
        })
    }
}

fn extract_ids(
    room: Option<&Room>,
    participant: Option<&ParticipantInfo>,
) -> Result<Option<(Uuid, Uuid)>, VoiceWebhookVerifierError> {
    let room = room.ok_or(VoiceWebhookVerifierError::MissingField("room"))?;
    let participant = participant.ok_or(VoiceWebhookVerifierError::MissingField("participant"))?;

    let channel_id = Uuid::parse_str(&room.name).map_err(|_| {
        VoiceWebhookVerifierError::InvalidIdentifier(format!(
            "room.name is not a uuid: {}",
            room.name
        ))
    })?;
    let Ok(user_id) = Uuid::parse_str(&participant.identity) else {
        return Ok(None);
    };
    Ok(Some((channel_id, user_id)))
}

fn parse_disconnect_reason(reason: i32) -> DisconnectReason {
    DisconnectReason::try_from(reason).unwrap_or(DisconnectReason::UnknownReason)
}

fn map_leave_reason(
    reason: DisconnectReason,
    channel_id: Uuid,
    user_id: Uuid,
    session_id: String,
) -> VoiceWebhookEvent {
    match reason {
        DisconnectReason::Migration => VoiceWebhookEvent::Other,

        DisconnectReason::SignalClose
        | DisconnectReason::MediaFailure
        | DisconnectReason::ConnectionTimeout
        | DisconnectReason::StateMismatch
        | DisconnectReason::JoinFailure
        | DisconnectReason::SipTrunkFailure => VoiceWebhookEvent::ParticipantConnectionAborted {
            channel_id,
            user_id,
            session_id,
            reason: abort_reason_from(reason),
        },

        DisconnectReason::UnknownReason
        | DisconnectReason::ClientInitiated
        | DisconnectReason::DuplicateIdentity
        | DisconnectReason::ServerShutdown
        | DisconnectReason::ParticipantRemoved
        | DisconnectReason::RoomDeleted
        | DisconnectReason::RoomClosed
        | DisconnectReason::UserUnavailable
        | DisconnectReason::UserRejected => VoiceWebhookEvent::ParticipantLeft {
            channel_id,
            user_id,
            session_id,
            reason: leave_reason_from(reason),
        },
    }
}

fn require_sid(participant: &ParticipantInfo) -> Result<String, VoiceWebhookVerifierError> {
    if participant.sid.is_empty() {
        return Err(VoiceWebhookVerifierError::MissingField("participant.sid"));
    }
    Ok(participant.sid.clone())
}

fn leave_reason_from(reason: DisconnectReason) -> LeaveReason {
    match reason {
        DisconnectReason::ClientInitiated => LeaveReason::ClientInitiated,
        DisconnectReason::ParticipantRemoved => LeaveReason::Kicked,
        DisconnectReason::DuplicateIdentity => LeaveReason::DuplicateIdentity,
        DisconnectReason::ServerShutdown => LeaveReason::ServerShutdown,
        DisconnectReason::RoomDeleted | DisconnectReason::RoomClosed => LeaveReason::RoomClosed,
        _ => LeaveReason::Unknown,
    }
}

fn abort_reason_from(reason: DisconnectReason) -> ConnectionAbortReason {
    match reason {
        DisconnectReason::SignalClose => ConnectionAbortReason::SignalClose,
        DisconnectReason::MediaFailure => ConnectionAbortReason::MediaFailure,
        DisconnectReason::ConnectionTimeout => ConnectionAbortReason::ConnectionTimeout,
        DisconnectReason::StateMismatch => ConnectionAbortReason::StateMismatch,
        DisconnectReason::JoinFailure => ConnectionAbortReason::JoinFailure,
        DisconnectReason::UnknownReason => ConnectionAbortReason::Unknown,
        _ => ConnectionAbortReason::Other,
    }
}

fn map_err(e: livekit_api::webhooks::WebhookError) -> VoiceWebhookVerifierError {
    match e {
        livekit_api::webhooks::WebhookError::InvalidSignature => {
            VoiceWebhookVerifierError::InvalidSignature
        }
        livekit_api::webhooks::WebhookError::InvalidAuth(e) => {
            VoiceWebhookVerifierError::InvalidBody(format!("auth: {}", e))
        }
        livekit_api::webhooks::WebhookError::InvalidBase64(e) => {
            VoiceWebhookVerifierError::InvalidBody(format!("base64: {}", e))
        }
        livekit_api::webhooks::WebhookError::InvalidData(e) => {
            VoiceWebhookVerifierError::InvalidBody(format!("json: {}", e))
        }
    }
}
