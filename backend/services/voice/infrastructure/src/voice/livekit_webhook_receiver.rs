use contracts::events::webhook::{ConnectionAbortReasonV1, LeaveReasonV1, MediaSourceV1};
use voice::ports::voice_webhook_verifier::{
    VerifiedVoiceWebhookEvent, VoiceWebhookEvent, VoiceWebhookVerifier, VoiceWebhookVerifierError,
    WebhookSource,
};
use chrono::{DateTime, Utc};
use livekit_api::{access_token::TokenVerifier, webhooks::WebhookReceiver};
use livekit_protocol::{DisconnectReason, ParticipantInfo, Room, TrackInfo, TrackSource};
use uuid::Uuid;

use infrastructure_common::config::livekit::LiveKitConfig;

const EVENT_PARTICIPANT_CONNECTION_ABORTED: &str = "participant_connection_aborted";
const EVENT_PARTICIPANT_LEFT: &str = "participant_left";
const EVENT_PARTICIPANT_JOINED: &str = "participant_joined";
const EVENT_TRACK_PUBLISHED: &str = "track_published";
const EVENT_TRACK_UNPUBLISHED: &str = "track_unpublished";

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
            EVENT_TRACK_PUBLISHED | EVENT_TRACK_UNPUBLISHED => {
                let participant = event
                    .participant
                    .as_ref()
                    .ok_or(VoiceWebhookVerifierError::MissingField("participant"))?;
                let source = event.track.as_ref().and_then(media_source_from);
                match (extract_ids(event.room.as_ref(), Some(participant))?, source) {
                    (Some((channel_id, user_id)), Some(source)) => {
                        let session_id = require_sid(participant)?;
                        if event.event.as_str() == EVENT_TRACK_PUBLISHED {
                            VoiceWebhookEvent::TrackPublished {
                                channel_id,
                                user_id,
                                session_id,
                                source,
                            }
                        } else {
                            VoiceWebhookEvent::TrackUnpublished {
                                channel_id,
                                user_id,
                                session_id,
                                source,
                            }
                        }
                    }
                    _ => VoiceWebhookEvent::Other,
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

fn media_source_from(track: &TrackInfo) -> Option<MediaSourceV1> {
    match TrackSource::try_from(track.source).ok()? {
        TrackSource::Camera => Some(MediaSourceV1::Camera),
        TrackSource::ScreenShare => Some(MediaSourceV1::Screen),
        _ => None,
    }
}

fn require_sid(participant: &ParticipantInfo) -> Result<String, VoiceWebhookVerifierError> {
    if participant.sid.is_empty() {
        return Err(VoiceWebhookVerifierError::MissingField("participant.sid"));
    }
    Ok(participant.sid.clone())
}

fn leave_reason_from(reason: DisconnectReason) -> LeaveReasonV1 {
    match reason {
        DisconnectReason::ClientInitiated => LeaveReasonV1::ClientInitiated,
        DisconnectReason::ParticipantRemoved => LeaveReasonV1::Kicked,
        DisconnectReason::DuplicateIdentity => LeaveReasonV1::DuplicateIdentity,
        DisconnectReason::ServerShutdown => LeaveReasonV1::ServerShutdown,
        DisconnectReason::RoomDeleted | DisconnectReason::RoomClosed => LeaveReasonV1::RoomClosed,
        _ => LeaveReasonV1::Unknown,
    }
}

fn abort_reason_from(reason: DisconnectReason) -> ConnectionAbortReasonV1 {
    match reason {
        DisconnectReason::SignalClose => ConnectionAbortReasonV1::SignalClose,
        DisconnectReason::MediaFailure => ConnectionAbortReasonV1::MediaFailure,
        DisconnectReason::ConnectionTimeout => ConnectionAbortReasonV1::ConnectionTimeout,
        DisconnectReason::StateMismatch => ConnectionAbortReasonV1::StateMismatch,
        DisconnectReason::JoinFailure => ConnectionAbortReasonV1::JoinFailure,
        DisconnectReason::UnknownReason => ConnectionAbortReasonV1::Unknown,
        _ => ConnectionAbortReasonV1::Other,
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
