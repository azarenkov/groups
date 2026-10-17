use api_common::api_error::{StatusCode, ToHttpStatus};

api_common::http_error_boilerplate!();

use voice::ports::voice_webhook_verifier::VoiceWebhookVerifierError;
use voice::usecases::call::accept_call::AcceptCallError;
use voice::usecases::call::add_participants::AddCallParticipantsError;
use voice::usecases::call::decline_call::DeclineCallError;
use voice::usecases::call::get_active_call::GetActiveCallError;
use voice::usecases::call::get_active_call_with::GetActiveCallWithError;
use voice::usecases::call::join_call::JoinCallError;
use voice::usecases::call::leave_call::LeaveCallError;
use voice::usecases::call::start_call::StartCallError;
use voice::usecases::webhook::voice::HandleVoiceWebhookError;

impl HttpMapping for VoiceWebhookVerifierError {
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

impl HttpMapping for HandleVoiceWebhookError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_http_status(),
            HandleVoiceWebhookError::Call(e) => e.to_http_status(),
            HandleVoiceWebhookError::Voice(e) => e.to_http_status(),
            HandleVoiceWebhookError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_user_message(),
            HandleVoiceWebhookError::Call(e) => e.to_user_message(),
            HandleVoiceWebhookError::Voice(e) => e.to_user_message(),
            HandleVoiceWebhookError::FriendshipLookup(_) => {
                "Friendship lookup unavailable".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_error_code(),
            HandleVoiceWebhookError::Call(e) => e.to_error_code(),
            HandleVoiceWebhookError::Voice(e) => e.to_error_code(),
            HandleVoiceWebhookError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}

impl HttpMapping for StartCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            StartCallError::Repository(e) => e.to_http_status(),
            StartCallError::Voice(e) => e.to_http_status(),
            StartCallError::NoRecipients => StatusCode::BAD_REQUEST,
            StartCallError::TooManyRecipients => StatusCode::BAD_REQUEST,
            StartCallError::CallNotAllowed => StatusCode::FORBIDDEN,
            StartCallError::UserNotFound => StatusCode::NOT_FOUND,
            StartCallError::AccountInactive => StatusCode::FORBIDDEN,
            StartCallError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            StartCallError::Repository(e) => e.to_user_message(),
            StartCallError::Voice(e) => e.to_user_message(),
            StartCallError::NoRecipients => "No recipients specified".to_string(),
            StartCallError::TooManyRecipients => "Too many recipients".to_string(),
            StartCallError::CallNotAllowed => "This user does not accept this call".to_string(),
            StartCallError::UserNotFound => "User not found".to_string(),
            StartCallError::AccountInactive => "Account is inactive".to_string(),
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
            StartCallError::TooManyRecipients => "CALL_TOO_MANY_RECIPIENTS",
            StartCallError::CallNotAllowed => "CALL_NOT_ALLOWED",
            StartCallError::UserNotFound => "USER_NOT_FOUND",
            StartCallError::AccountInactive => "ACCOUNT_INACTIVE",
            StartCallError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}

impl HttpMapping for AcceptCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AcceptCallError::Repository(e) => e.to_http_status(),
            AcceptCallError::Voice(e) => e.to_http_status(),
            AcceptCallError::NotParticipant => StatusCode::FORBIDDEN,
            AcceptCallError::UserNotFound => StatusCode::NOT_FOUND,
            AcceptCallError::AccountInactive => StatusCode::FORBIDDEN,
            AcceptCallError::CallNotAllowed => StatusCode::FORBIDDEN,
            AcceptCallError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            AcceptCallError::Repository(e) => e.to_user_message(),
            AcceptCallError::Voice(e) => e.to_user_message(),
            AcceptCallError::NotParticipant => "You are not invited to this call".to_string(),
            AcceptCallError::UserNotFound => "User not found".to_string(),
            AcceptCallError::AccountInactive => "Account is inactive".to_string(),
            AcceptCallError::CallNotAllowed => "This user does not accept this call".to_string(),
            AcceptCallError::FriendshipLookup(_) => {
                "Friendship lookup unavailable".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            AcceptCallError::Repository(e) => e.to_error_code(),
            AcceptCallError::Voice(e) => e.to_error_code(),
            AcceptCallError::NotParticipant => "CALL_NOT_PARTICIPANT",
            AcceptCallError::UserNotFound => "USER_NOT_FOUND",
            AcceptCallError::AccountInactive => "ACCOUNT_INACTIVE",
            AcceptCallError::CallNotAllowed => "CALL_NOT_ALLOWED",
            AcceptCallError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}

impl HttpMapping for DeclineCallError {
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

impl HttpMapping for JoinCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinCallError::Repository(e) => e.to_http_status(),
            JoinCallError::Voice(e) => e.to_http_status(),
            JoinCallError::NotParticipant => StatusCode::FORBIDDEN,
            JoinCallError::CallFull => StatusCode::CONFLICT,
            JoinCallError::UserNotFound => StatusCode::NOT_FOUND,
            JoinCallError::AccountInactive => StatusCode::FORBIDDEN,
            JoinCallError::CallNotAllowed => StatusCode::FORBIDDEN,
            JoinCallError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            JoinCallError::Repository(e) => e.to_user_message(),
            JoinCallError::Voice(e) => e.to_user_message(),
            JoinCallError::NotParticipant => "You are not part of this call".to_string(),
            JoinCallError::CallFull => "This call is full".to_string(),
            JoinCallError::UserNotFound => "User not found".to_string(),
            JoinCallError::AccountInactive => "Account is inactive".to_string(),
            JoinCallError::CallNotAllowed => "This user does not accept this call".to_string(),
            JoinCallError::FriendshipLookup(_) => {
                "Friendship lookup unavailable".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            JoinCallError::Repository(e) => e.to_error_code(),
            JoinCallError::Voice(e) => e.to_error_code(),
            JoinCallError::NotParticipant => "CALL_NOT_PARTICIPANT",
            JoinCallError::CallFull => "CALL_FULL",
            JoinCallError::UserNotFound => "USER_NOT_FOUND",
            JoinCallError::AccountInactive => "ACCOUNT_INACTIVE",
            JoinCallError::CallNotAllowed => "CALL_NOT_ALLOWED",
            JoinCallError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}

impl HttpMapping for GetActiveCallWithError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetActiveCallWithError::Repository(e) => e.to_http_status(),
            GetActiveCallWithError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetActiveCallWithError::Repository(e) => e.to_user_message(),
            GetActiveCallWithError::FriendshipLookup(_) => {
                "Friendship lookup unavailable".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetActiveCallWithError::Repository(e) => e.to_error_code(),
            GetActiveCallWithError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}

impl HttpMapping for GetActiveCallError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetActiveCallError::Repository(e) => e.to_http_status(),
            GetActiveCallError::Voice(e) => e.to_http_status(),
            GetActiveCallError::UserNotFound => StatusCode::NOT_FOUND,
            GetActiveCallError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetActiveCallError::Repository(e) => e.to_user_message(),
            GetActiveCallError::Voice(e) => e.to_user_message(),
            GetActiveCallError::UserNotFound => "User not found".to_string(),
            GetActiveCallError::FriendshipLookup(_) => {
                "Friendship lookup unavailable".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetActiveCallError::Repository(e) => e.to_error_code(),
            GetActiveCallError::Voice(e) => e.to_error_code(),
            GetActiveCallError::UserNotFound => "USER_NOT_FOUND",
            GetActiveCallError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}

impl HttpMapping for LeaveCallError {
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

impl HttpMapping for AddCallParticipantsError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AddCallParticipantsError::Repository(e) => e.to_http_status(),
            AddCallParticipantsError::NotParticipant => StatusCode::FORBIDDEN,
            AddCallParticipantsError::CallNotAllowed => StatusCode::FORBIDDEN,
            AddCallParticipantsError::NoRecipients => StatusCode::BAD_REQUEST,
            AddCallParticipantsError::TooManyRecipients => StatusCode::BAD_REQUEST,
            AddCallParticipantsError::FriendshipLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            AddCallParticipantsError::Repository(e) => e.to_user_message(),
            AddCallParticipantsError::NotParticipant => {
                "You are not part of this call".to_string()
            }
            AddCallParticipantsError::CallNotAllowed => {
                "This user cannot be added to this call".to_string()
            }
            AddCallParticipantsError::NoRecipients => "No recipients specified".to_string(),
            AddCallParticipantsError::TooManyRecipients => "Too many recipients".to_string(),
            AddCallParticipantsError::FriendshipLookup(_) => {
                "Friendship lookup unavailable".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            AddCallParticipantsError::Repository(e) => e.to_error_code(),
            AddCallParticipantsError::NotParticipant => "CALL_NOT_PARTICIPANT",
            AddCallParticipantsError::CallNotAllowed => "CALL_NOT_ALLOWED",
            AddCallParticipantsError::NoRecipients => "CALL_NO_RECIPIENTS",
            AddCallParticipantsError::TooManyRecipients => "CALL_TOO_MANY_RECIPIENTS",
            AddCallParticipantsError::FriendshipLookup(_) => "FRIENDSHIP_LOOKUP_UNAVAILABLE",
        }
    }
}
