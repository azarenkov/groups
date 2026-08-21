use shared::api_error::{StatusCode, ToHttpStatus};
use crate::ports::voice_webhook_verifier::VoiceWebhookVerifierError;
use crate::usecases::call::accept_call::AcceptCallError;
use crate::usecases::call::add_participants::AddCallParticipantsError;
use crate::usecases::call::decline_call::DeclineCallError;
use crate::usecases::call::get_active_call::GetActiveCallError;
use crate::usecases::call::get_active_call_with::GetActiveCallWithError;
use crate::usecases::call::join_call::JoinCallError;
use crate::usecases::call::leave_call::LeaveCallError;
use crate::usecases::call::start_call::StartCallError;
use crate::usecases::webhook::voice::HandleVoiceWebhookError;

impl ToHttpStatus for VoiceWebhookVerifierError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            VoiceWebhookVerifierError::InvalidSignature => StatusCode::UNAUTHORIZED,
            VoiceWebhookVerifierError::InvalidBody(_)
            | VoiceWebhookVerifierError::MissingField(_)
            | VoiceWebhookVerifierError::InvalidIdentifier(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            VoiceWebhookVerifierError::InvalidSignature => "Invalid signature".to_string(),
            VoiceWebhookVerifierError::InvalidBody(_) => "Invalid webhook body".to_string(),
            VoiceWebhookVerifierError::MissingField(f) => {
                format!("Missing required webhook field: {f}")
            }
            VoiceWebhookVerifierError::InvalidIdentifier(_) => {
                "Invalid identifier in webhook".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            VoiceWebhookVerifierError::InvalidSignature => "WEBHOOK_INVALID_SIGNATURE",
            VoiceWebhookVerifierError::InvalidBody(_) => "WEBHOOK_INVALID_BODY",
            VoiceWebhookVerifierError::MissingField(_) => "WEBHOOK_MISSING_FIELD",
            VoiceWebhookVerifierError::InvalidIdentifier(_) => "WEBHOOK_INVALID_IDENTIFIER",
        }
    }
}

impl ToHttpStatus for HandleVoiceWebhookError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_http_status(),
            HandleVoiceWebhookError::Call(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_user_message(),
            HandleVoiceWebhookError::Call(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_error_code(),
            HandleVoiceWebhookError::Call(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for StartCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            StartCallError::Repository(e) => e.to_http_status(),
            StartCallError::Voice(e) => e.to_http_status(),
            StartCallError::NoRecipients => StatusCode::BAD_REQUEST,
            StartCallError::NotFriends => StatusCode::FORBIDDEN,
            StartCallError::UserNotFound => StatusCode::NOT_FOUND,
            StartCallError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            StartCallError::Repository(e) => e.to_user_message(),
            StartCallError::Voice(e) => e.to_user_message(),
            StartCallError::NoRecipients => "No recipients specified".to_string(),
            StartCallError::NotFriends => "You can only call your friends".to_string(),
            StartCallError::UserNotFound => "User not found".to_string(),
            StartCallError::FriendshipLookup(_) => {
                "Friendship lookup unavailable".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            StartCallError::Repository(e) => e.to_error_code(),
            StartCallError::Voice(e) => e.to_error_code(),
            StartCallError::NoRecipients => "CALL_NO_RECIPIENTS",
            StartCallError::NotFriends => "CALL_NOT_FRIENDS",
            StartCallError::UserNotFound => "USER_NOT_FOUND",
            StartCallError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}

impl ToHttpStatus for AcceptCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AcceptCallError::Repository(e) => e.to_http_status(),
            AcceptCallError::Voice(e) => e.to_http_status(),
            AcceptCallError::NotParticipant => StatusCode::FORBIDDEN,
            AcceptCallError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            AcceptCallError::Repository(e) => e.to_user_message(),
            AcceptCallError::Voice(e) => e.to_user_message(),
            AcceptCallError::NotParticipant => "You are not invited to this call".to_string(),
            AcceptCallError::UserNotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            AcceptCallError::Repository(e) => e.to_error_code(),
            AcceptCallError::Voice(e) => e.to_error_code(),
            AcceptCallError::NotParticipant => "CALL_NOT_PARTICIPANT",
            AcceptCallError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl ToHttpStatus for DeclineCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeclineCallError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DeclineCallError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DeclineCallError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for JoinCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinCallError::Repository(e) => e.to_http_status(),
            JoinCallError::Voice(e) => e.to_http_status(),
            JoinCallError::NotParticipant => StatusCode::FORBIDDEN,
            JoinCallError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            JoinCallError::Repository(e) => e.to_user_message(),
            JoinCallError::Voice(e) => e.to_user_message(),
            JoinCallError::NotParticipant => "You are not part of this call".to_string(),
            JoinCallError::UserNotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            JoinCallError::Repository(e) => e.to_error_code(),
            JoinCallError::Voice(e) => e.to_error_code(),
            JoinCallError::NotParticipant => "CALL_NOT_PARTICIPANT",
            JoinCallError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl ToHttpStatus for GetActiveCallWithError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetActiveCallWithError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetActiveCallWithError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetActiveCallWithError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for GetActiveCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetActiveCallError::Repository(e) => e.to_http_status(),
            GetActiveCallError::Voice(e) => e.to_http_status(),
            GetActiveCallError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetActiveCallError::Repository(e) => e.to_user_message(),
            GetActiveCallError::Voice(e) => e.to_user_message(),
            GetActiveCallError::UserNotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetActiveCallError::Repository(e) => e.to_error_code(),
            GetActiveCallError::Voice(e) => e.to_error_code(),
            GetActiveCallError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl ToHttpStatus for LeaveCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LeaveCallError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            LeaveCallError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            LeaveCallError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for AddCallParticipantsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AddCallParticipantsError::Repository(e) => e.to_http_status(),
            AddCallParticipantsError::NotParticipant => StatusCode::FORBIDDEN,
            AddCallParticipantsError::NotFriends => StatusCode::FORBIDDEN,
            AddCallParticipantsError::NoRecipients => StatusCode::BAD_REQUEST,
            AddCallParticipantsError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            AddCallParticipantsError::Repository(e) => e.to_user_message(),
            AddCallParticipantsError::NotParticipant => {
                "You are not part of this call".to_string()
            }
            AddCallParticipantsError::NotFriends => {
                "You can only invite your friends".to_string()
            }
            AddCallParticipantsError::NoRecipients => "No recipients specified".to_string(),
            AddCallParticipantsError::FriendshipLookup(_) => {
                "Friendship lookup unavailable".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            AddCallParticipantsError::Repository(e) => e.to_error_code(),
            AddCallParticipantsError::NotParticipant => "CALL_NOT_PARTICIPANT",
            AddCallParticipantsError::NotFriends => "CALL_NOT_FRIENDS",
            AddCallParticipantsError::NoRecipients => "CALL_NO_RECIPIENTS",
            AddCallParticipantsError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}

shared::impl_from_for_api_error!(
    AcceptCallError,
    AddCallParticipantsError,
    DeclineCallError,
    GetActiveCallError,
    GetActiveCallWithError,
    HandleVoiceWebhookError,
    JoinCallError,
    LeaveCallError,
    StartCallError,
);
