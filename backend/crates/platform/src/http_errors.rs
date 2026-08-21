use shared::api_error::{StatusCode, ToHttpStatus};

use crate::ports::file_storage::FileStorageError;
use crate::ports::hasher::HashError;
use crate::ports::mailer::MailerError;
use crate::ports::voice_service::VoiceServiceError;

impl ToHttpStatus for VoiceServiceError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Voice service error".to_string()
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            VoiceServiceError::TokenCreation(_) => "VOICE_TOKEN_CREATION_FAILED",
            VoiceServiceError::VoiceChannelCreation(_) => "VOICE_CHANNEL_CREATION_FAILED",
            VoiceServiceError::VoiceChannelDeletion(_) => "VOICE_CHANNEL_DELETION_FAILED",
            VoiceServiceError::ParticipantRemoval(_) => "VOICE_PARTICIPANT_REMOVAL_FAILED",
        }
    }
}

impl ToHttpStatus for HashError {
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

impl ToHttpStatus for MailerError {
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

impl ToHttpStatus for FileStorageError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            FileStorageError::NotFound => StatusCode::NOT_FOUND,
            FileStorageError::Internal(_) => StatusCode::BAD_GATEWAY,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            FileStorageError::NotFound => "File not found.".to_string(),
            FileStorageError::Internal(_) => {
                "File storage is temporarily unavailable.".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            FileStorageError::NotFound => "ATTACHMENT_NOT_FOUND",
            FileStorageError::Internal(_) => "STORAGE_UNAVAILABLE",
        }
    }
}
