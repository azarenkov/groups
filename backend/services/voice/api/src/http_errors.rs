use actix_web::{HttpResponse, ResponseError};
use api_common::api_error::{ApiError, StatusCode, ToHttpStatus};

pub trait HttpMapping {
    fn to_http_status(&self) -> StatusCode;
    fn to_user_message(&self) -> String;
    fn to_error_code(&self) -> &'static str;
}

#[derive(Debug)]
pub struct HttpError(pub ApiError);

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for HttpError {
    fn status_code(&self) -> StatusCode {
        self.0.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        self.0.error_response()
    }
}

#[allow(dead_code)]
impl HttpError {
    pub fn new(message: String, status: StatusCode, code: &'static str) -> Self {
        Self(ApiError::new(message, status, code))
    }

    pub fn bad_request(message: String) -> Self {
        Self(ApiError::bad_request(message))
    }

    pub fn payload_too_large() -> Self {
        Self(ApiError::payload_too_large())
    }

    pub fn from_error<E: ToHttpStatus + std::fmt::Debug>(error: E) -> Self {
        Self(ApiError::from_error(error))
    }
}

impl From<ApiError> for HttpError {
    fn from(error: ApiError) -> Self {
        Self(error)
    }
}

impl From<validator::ValidationErrors> for HttpError {
    fn from(error: validator::ValidationErrors) -> Self {
        Self(ApiError::from(error))
    }
}

impl From<domain::shared::repository::error::RepositoryError> for HttpError {
    fn from(error: domain::shared::repository::error::RepositoryError) -> Self {
        Self(ApiError::from(error))
    }
}

impl From<domain::shared::pagination::PaginationError> for HttpError {
    fn from(error: domain::shared::pagination::PaginationError) -> Self {
        Self(ApiError::from(error))
    }
}

impl<E: HttpMapping + std::fmt::Debug> From<E> for HttpError {
    fn from(error: E) -> Self {
        tracing::error!("API Error: {:?}", error);
        Self(ApiError::new(
            error.to_user_message(),
            error.to_http_status(),
            error.to_error_code(),
        ))
    }
}

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

impl HttpMapping for StartCallError {
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

impl HttpMapping for AcceptCallError {
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

impl HttpMapping for GetActiveCallWithError {
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

impl HttpMapping for GetActiveCallError {
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


impl HttpMapping for ports::voice_service::VoiceServiceError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Voice service error".to_string()
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ports::voice_service::VoiceServiceError::TokenCreation(_) => "VOICE_TOKEN_CREATION_FAILED",
            ports::voice_service::VoiceServiceError::VoiceChannelCreation(_) => "VOICE_CHANNEL_CREATION_FAILED",
            ports::voice_service::VoiceServiceError::VoiceChannelDeletion(_) => "VOICE_CHANNEL_DELETION_FAILED",
            ports::voice_service::VoiceServiceError::ParticipantRemoval(_) => "VOICE_PARTICIPANT_REMOVAL_FAILED",
        }
    }
}

impl HttpMapping for ports::hasher::HashError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Authentication error".to_string()
    }

    fn to_error_code(&self) -> &'static str {
        "INTERNAL_ERROR"
    }
}

impl HttpMapping for ports::mailer::MailerError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::BAD_GATEWAY
    }

    fn to_user_message(&self) -> String {
        "Could not send email. Please try again in a moment.".to_string()
    }

    fn to_error_code(&self) -> &'static str {
        "MAILER_UNAVAILABLE"
    }
}

impl HttpMapping for ports::file_storage::FileStorageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ports::file_storage::FileStorageError::NotFound => StatusCode::NOT_FOUND,
            ports::file_storage::FileStorageError::Internal(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ports::file_storage::FileStorageError::NotFound => "File not found.".to_string(),
            ports::file_storage::FileStorageError::Internal(_) => {
                "File storage is temporarily unavailable.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ports::file_storage::FileStorageError::NotFound => "ATTACHMENT_NOT_FOUND",
            ports::file_storage::FileStorageError::Internal(_) => "STORAGE_UNAVAILABLE",
        }
    }
}
