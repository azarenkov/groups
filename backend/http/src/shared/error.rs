use std::borrow::Cow;

use actix_web::{
    HttpResponse, ResponseError,
    http::{StatusCode, header::HeaderValue},
};
use application::ports::{
    hasher::HashError, mailer::MailerError, token_issuer::TokenIssuerError,
    voice_service::VoiceServiceError, voice_webhook_verifier::VoiceWebhookVerifierError,
};
use application::usecases::channel::join_voice_channel::JoinVoiceChannelError;
use application::usecases::server::list_server_channels::ListServerChannelsError;
use application::usecases::server::list_server_members::ListServerMembersError;
use application::usecases::server::list_servers_by_user::ListServersByUserError;
use application::usecases::{
    auth::{
        confirm_registration::ConfirmRegistrationError,
        issue_verification_code::IssueVerificationCodeError, login::LoginError,
        logout::LogoutError, refresh_tokens::RefreshTokensError, register::RegistrationError,
    },
    channel::{
        abort_voice_connection::AbortVoiceConnectionError, create_channel::CreateChannelError,
        delete_channel::DeleteChannelError, join_voice_connection::JoinVoiceConnectionError,
        leave_voice_connection::LeaveVoiceConnectionError, mute_channel::MuteChannelError,
        unmute_channel::UnmuteChannelError,
    },
    server::{create_server::CreateServerError, delete_server::DeleteServerError},
    webhook::voice::HandleVoiceWebhookError,
};
use domain::channel::ChannelError;
use domain::shared::outbox::to_outbox::OutboxError;
use domain::shared::pagination::PaginationError;
use domain::shared::repository::error::RepositoryError;
use serde::Serialize;
use validator::{ValidationErrors, ValidationErrorsKind};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    code: &'static str,
    message: String,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<FieldError>>,
}

pub trait ToHttpStatus {
    fn to_http_status(&self) -> StatusCode;
    fn to_user_message(&self) -> String;
    fn to_error_code(&self) -> &'static str;
}

impl ToHttpStatus for RepositoryError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RepositoryError::NotFound => StatusCode::NOT_FOUND,
            RepositoryError::AlreadyExists => StatusCode::CONFLICT,
            RepositoryError::ConnectionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RepositoryError::InvalidData(_) => StatusCode::BAD_REQUEST,
            RepositoryError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RepositoryError::NotFound => "Resource not found".to_string(),
            RepositoryError::AlreadyExists => "Resource already exists".to_string(),
            RepositoryError::ConnectionError(_) => "Service temporarily unavailable".to_string(),
            RepositoryError::InvalidData(_) => "Invalid data provided".to_string(),
            RepositoryError::Internal(_) => "Internal server error".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            RepositoryError::NotFound => "RESOURCE_NOT_FOUND",
            RepositoryError::AlreadyExists => "RESOURCE_ALREADY_EXISTS",
            RepositoryError::ConnectionError(_) => "SERVICE_UNAVAILABLE",
            RepositoryError::InvalidData(_) => "INVALID_DATA",
            RepositoryError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

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

impl ToHttpStatus for ChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ChannelError::PrivateChannelRequiresPassword
            | ChannelError::PublicChannelCannotHavePassword
            | ChannelError::NameUnchanged => StatusCode::BAD_REQUEST,
            ChannelError::NotOwner => StatusCode::FORBIDDEN,
            ChannelError::AlreadyMuted | ChannelError::NotMuted => StatusCode::CONFLICT,
            ChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ChannelError::PrivateChannelRequiresPassword => {
                "Private channel requires a password".to_string()
            }
            ChannelError::PublicChannelCannotHavePassword => {
                "Public channel cannot have a password".to_string()
            }
            ChannelError::NameUnchanged => "Channel name is unchanged".to_string(),
            ChannelError::NotOwner => "Only the channel owner can perform this action".to_string(),
            ChannelError::AlreadyMuted => "Participant is already muted".to_string(),
            ChannelError::NotMuted => "Participant is not muted".to_string(),
            ChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ChannelError::PrivateChannelRequiresPassword => "CHANNEL_PASSWORD_REQUIRED",
            ChannelError::PublicChannelCannotHavePassword => "CHANNEL_PASSWORD_NOT_ALLOWED",
            ChannelError::NameUnchanged => "CHANNEL_NAME_UNCHANGED",
            ChannelError::NotOwner => "CHANNEL_NOT_OWNER",
            ChannelError::AlreadyMuted => "PARTICIPANT_ALREADY_MUTED",
            ChannelError::NotMuted => "PARTICIPANT_NOT_MUTED",
            ChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
        }
    }
}

impl ToHttpStatus for OutboxError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Internal server error".to_string()
    }

    fn to_error_code(&self) -> &'static str {
        "INTERNAL_ERROR"
    }
}

impl ToHttpStatus for PaginationError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn to_user_message(&self) -> String {
        self.to_string()
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            PaginationError::InvalidPage => "PAGINATION_INVALID_PAGE",
            PaginationError::InvalidPerPage(_) => "PAGINATION_INVALID_PER_PAGE",
        }
    }
}

impl ToHttpStatus for CreateChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateChannelError::Repository(e) => e.to_http_status(),
            CreateChannelError::Hash(e) => e.to_http_status(),
            CreateChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            CreateChannelError::InvalidData => StatusCode::BAD_REQUEST,
            CreateChannelError::Domain(e) => e.to_http_status(),
            CreateChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            CreateChannelError::Repository(e) => e.to_user_message(),
            CreateChannelError::Hash(e) => e.to_user_message(),
            CreateChannelError::PasswordRequired => {
                "Password is required for private channels".to_string()
            }
            CreateChannelError::InvalidData => "Invalid channel data provided".to_string(),
            CreateChannelError::Domain(e) => e.to_user_message(),
            CreateChannelError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            CreateChannelError::Repository(e) => e.to_error_code(),
            CreateChannelError::Hash(e) => e.to_error_code(),
            CreateChannelError::PasswordRequired => "CHANNEL_PASSWORD_REQUIRED",
            CreateChannelError::InvalidData => "CHANNEL_INVALID_DATA",
            CreateChannelError::Domain(e) => e.to_error_code(),
            CreateChannelError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for DeleteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteChannelError::ChannelNotFound => StatusCode::NOT_FOUND,
            DeleteChannelError::NotOwner => StatusCode::FORBIDDEN,
            DeleteChannelError::Repository(e) => e.to_http_status(),
            DeleteChannelError::Domain(e) => e.to_http_status(),
            DeleteChannelError::Outbox(e) => e.to_http_status(),
            DeleteChannelError::VoiceService(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteChannelError::ChannelNotFound => "Channel not found".to_string(),
            DeleteChannelError::NotOwner => "Only the channel owner can delete it.".to_string(),
            DeleteChannelError::Repository(e) => e.to_user_message(),
            DeleteChannelError::Domain(e) => e.to_user_message(),
            DeleteChannelError::Outbox(e) => e.to_user_message(),
            DeleteChannelError::VoiceService(_) => {
                "Voice service is temporarily unavailable.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            DeleteChannelError::NotOwner => "CHANNEL_NOT_OWNER",
            DeleteChannelError::Repository(e) => e.to_error_code(),
            DeleteChannelError::Domain(e) => e.to_error_code(),
            DeleteChannelError::Outbox(e) => e.to_error_code(),
            DeleteChannelError::VoiceService(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for MuteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            MuteChannelError::ChannelNotFound | MuteChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            MuteChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            MuteChannelError::Repository(e) => e.to_http_status(),
            MuteChannelError::Domain(e) => e.to_http_status(),
            MuteChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            MuteChannelError::ChannelNotFound => "Channel not found".to_string(),
            MuteChannelError::TargetUserNotFound => "Target user not found".to_string(),
            MuteChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            MuteChannelError::Repository(e) => e.to_user_message(),
            MuteChannelError::Domain(e) => e.to_user_message(),
            MuteChannelError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            MuteChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            MuteChannelError::TargetUserNotFound => "TARGET_USER_NOT_FOUND",
            MuteChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            MuteChannelError::Repository(e) => e.to_error_code(),
            MuteChannelError::Domain(e) => e.to_error_code(),
            MuteChannelError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for UnmuteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UnmuteChannelError::ChannelNotFound | UnmuteChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            UnmuteChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            UnmuteChannelError::Repository(e) => e.to_http_status(),
            UnmuteChannelError::Domain(e) => e.to_http_status(),
            UnmuteChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            UnmuteChannelError::ChannelNotFound => "Channel not found".to_string(),
            UnmuteChannelError::TargetUserNotFound => "Target user not found".to_string(),
            UnmuteChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            UnmuteChannelError::Repository(e) => e.to_user_message(),
            UnmuteChannelError::Domain(e) => e.to_user_message(),
            UnmuteChannelError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UnmuteChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            UnmuteChannelError::TargetUserNotFound => "TARGET_USER_NOT_FOUND",
            UnmuteChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            UnmuteChannelError::Repository(e) => e.to_error_code(),
            UnmuteChannelError::Domain(e) => e.to_error_code(),
            UnmuteChannelError::Outbox(e) => e.to_error_code(),
        }
    }
}

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

impl ToHttpStatus for AbortVoiceConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AbortVoiceConnectionError::Repository(e) => e.to_http_status(),
            AbortVoiceConnectionError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            AbortVoiceConnectionError::Repository(e) => e.to_user_message(),
            AbortVoiceConnectionError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            AbortVoiceConnectionError::Repository(e) => e.to_error_code(),
            AbortVoiceConnectionError::Domain(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for LeaveVoiceConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LeaveVoiceConnectionError::Repository(e) => e.to_http_status(),
            LeaveVoiceConnectionError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LeaveVoiceConnectionError::Repository(e) => e.to_user_message(),
            LeaveVoiceConnectionError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            LeaveVoiceConnectionError::Repository(e) => e.to_error_code(),
            LeaveVoiceConnectionError::Domain(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for JoinVoiceConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinVoiceConnectionError::Repository(e) => e.to_http_status(),
            JoinVoiceConnectionError::Domain(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            JoinVoiceConnectionError::Repository(e) => e.to_user_message(),
            JoinVoiceConnectionError::Domain(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            JoinVoiceConnectionError::Repository(e) => e.to_error_code(),
            JoinVoiceConnectionError::Domain(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for HandleVoiceWebhookError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_http_status(),
            HandleVoiceWebhookError::JoinVoiceConnection(e) => e.to_http_status(),
            HandleVoiceWebhookError::AbortVoiceConnection(e) => e.to_http_status(),
            HandleVoiceWebhookError::LeaveVoiceConnection(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_user_message(),
            HandleVoiceWebhookError::JoinVoiceConnection(e) => e.to_user_message(),
            HandleVoiceWebhookError::AbortVoiceConnection(e) => e.to_user_message(),
            HandleVoiceWebhookError::LeaveVoiceConnection(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_error_code(),
            HandleVoiceWebhookError::JoinVoiceConnection(e) => e.to_error_code(),
            HandleVoiceWebhookError::AbortVoiceConnection(e) => e.to_error_code(),
            HandleVoiceWebhookError::LeaveVoiceConnection(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for TokenIssuerError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            TokenIssuerError::Encoding(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TokenIssuerError::Decoding(_) | TokenIssuerError::InvalidTokenType => {
                StatusCode::UNAUTHORIZED
            }
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            TokenIssuerError::Encoding(_) => "Internal server error".to_string(),
            TokenIssuerError::Decoding(_) | TokenIssuerError::InvalidTokenType => {
                "Invalid refresh token".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            TokenIssuerError::Encoding(_) => "TOKEN_ENCODING_FAILED",
            TokenIssuerError::Decoding(_) => "TOKEN_DECODING_FAILED",
            TokenIssuerError::InvalidTokenType => "TOKEN_INVALID_TYPE",
        }
    }
}

impl ToHttpStatus for LoginError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LoginError::Repository(e) => e.to_http_status(),
            LoginError::Hash(e) => e.to_http_status(),
            LoginError::Outbox(e) => e.to_http_status(),
            LoginError::TokenIssuer(e) => e.to_http_status(),
            LoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LoginError::Repository(e) => e.to_user_message(),
            LoginError::Hash(e) => e.to_user_message(),
            LoginError::Outbox(e) => e.to_user_message(),
            LoginError::TokenIssuer(e) => e.to_user_message(),
            LoginError::InvalidCredentials => "Invalid credentials".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            LoginError::Repository(e) => e.to_error_code(),
            LoginError::Hash(e) => e.to_error_code(),
            LoginError::Outbox(e) => e.to_error_code(),
            LoginError::TokenIssuer(e) => e.to_error_code(),
            LoginError::InvalidCredentials => "INVALID_CREDENTIALS",
        }
    }
}

impl ToHttpStatus for RefreshTokensError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RefreshTokensError::Repository(e) => e.to_http_status(),
            RefreshTokensError::TokenIssuer(e) => e.to_http_status(),
            RefreshTokensError::RefreshTokenNotFound | RefreshTokensError::UserNotFound => {
                StatusCode::NOT_FOUND
            }
            RefreshTokensError::RefreshTokenRevoked
            | RefreshTokensError::RefreshTokenExpired
            | RefreshTokensError::UserMismatch => StatusCode::UNAUTHORIZED,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RefreshTokensError::Repository(e) => e.to_user_message(),
            RefreshTokensError::TokenIssuer(e) => e.to_user_message(),
            RefreshTokensError::RefreshTokenNotFound => "Refresh token not found".to_string(),
            RefreshTokensError::RefreshTokenRevoked => "Refresh token has been revoked".to_string(),
            RefreshTokensError::RefreshTokenExpired => "Refresh token has expired".to_string(),
            RefreshTokensError::UserMismatch => "Refresh token user mismatch".to_string(),
            RefreshTokensError::UserNotFound => "User not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            RefreshTokensError::Repository(e) => e.to_error_code(),
            RefreshTokensError::TokenIssuer(e) => e.to_error_code(),
            RefreshTokensError::RefreshTokenNotFound => "REFRESH_TOKEN_NOT_FOUND",
            RefreshTokensError::RefreshTokenRevoked => "REFRESH_TOKEN_REVOKED",
            RefreshTokensError::RefreshTokenExpired => "REFRESH_TOKEN_EXPIRED",
            RefreshTokensError::UserMismatch => "REFRESH_TOKEN_USER_MISMATCH",
            RefreshTokensError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl ToHttpStatus for LogoutError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LogoutError::Repository(e) => e.to_http_status(),
            LogoutError::Outbox(e) => e.to_http_status(),
            LogoutError::TokenIssuer(e) => e.to_http_status(),
            LogoutError::RefreshTokenNotFound => StatusCode::NOT_FOUND,
            LogoutError::UserMismatch => StatusCode::UNAUTHORIZED,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LogoutError::Repository(e) => e.to_user_message(),
            LogoutError::Outbox(e) => e.to_user_message(),
            LogoutError::TokenIssuer(e) => e.to_user_message(),
            LogoutError::RefreshTokenNotFound => "Refresh token not found".to_string(),
            LogoutError::UserMismatch => "Refresh token user mismatch".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            LogoutError::Repository(e) => e.to_error_code(),
            LogoutError::Outbox(e) => e.to_error_code(),
            LogoutError::TokenIssuer(e) => e.to_error_code(),
            LogoutError::RefreshTokenNotFound => "REFRESH_TOKEN_NOT_FOUND",
            LogoutError::UserMismatch => "REFRESH_TOKEN_USER_MISMATCH",
        }
    }
}

impl ToHttpStatus for ConfirmRegistrationError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ConfirmRegistrationError::UserAlreadyVerified => StatusCode::CONFLICT,
            ConfirmRegistrationError::TooManyAttempts => StatusCode::TOO_MANY_REQUESTS,
            ConfirmRegistrationError::VerificationCodeExpired => StatusCode::GONE,
            ConfirmRegistrationError::BadVerificationCode => StatusCode::BAD_REQUEST,
            ConfirmRegistrationError::NotRegistration => StatusCode::BAD_REQUEST,
            ConfirmRegistrationError::UserNotFound => StatusCode::NOT_FOUND,
            ConfirmRegistrationError::Repository(e) => e.to_http_status(),
            ConfirmRegistrationError::Hash(e) => e.to_http_status(),
            ConfirmRegistrationError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            ConfirmRegistrationError::UserAlreadyVerified => "User is already verified".to_string(),
            ConfirmRegistrationError::TooManyAttempts => {
                "Too many attempts for verification code".to_string()
            }
            ConfirmRegistrationError::VerificationCodeExpired => {
                "Verification code has expired".to_string()
            }
            ConfirmRegistrationError::BadVerificationCode => {
                "Invalid verification code".to_string()
            }
            ConfirmRegistrationError::NotRegistration => {
                "Verification code is not for registration".to_string()
            }
            ConfirmRegistrationError::UserNotFound => "User not found".to_string(),
            ConfirmRegistrationError::Repository(e) => e.to_user_message(),
            ConfirmRegistrationError::Hash(e) => e.to_user_message(),
            ConfirmRegistrationError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            ConfirmRegistrationError::UserAlreadyVerified => "USER_ALREADY_VERIFIED",
            ConfirmRegistrationError::TooManyAttempts => "VERIFICATION_TOO_MANY_ATTEMPTS",
            ConfirmRegistrationError::VerificationCodeExpired => "VERIFICATION_CODE_EXPIRED",
            ConfirmRegistrationError::BadVerificationCode => "VERIFICATION_CODE_INVALID",
            ConfirmRegistrationError::NotRegistration => "VERIFICATION_CODE_NOT_REGISTRATION",
            ConfirmRegistrationError::UserNotFound => "USER_NOT_FOUND",
            ConfirmRegistrationError::Repository(e) => e.to_error_code(),
            ConfirmRegistrationError::Hash(e) => e.to_error_code(),
            ConfirmRegistrationError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for RegistrationError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RegistrationError::Repository(e) => e.to_http_status(),
            RegistrationError::Hash(e) => e.to_http_status(),
            RegistrationError::Outbox(e) => e.to_http_status(),
            RegistrationError::TokenIssuer(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RegistrationError::Repository(e) => e.to_user_message(),
            RegistrationError::Hash(e) => e.to_user_message(),
            RegistrationError::Outbox(e) => e.to_user_message(),
            RegistrationError::TokenIssuer(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            RegistrationError::Repository(e) => match e {
                RepositoryError::AlreadyExists => "EMAIL_ALREADY_TAKEN",
                _ => e.to_error_code(),
            },
            RegistrationError::Hash(e) => e.to_error_code(),
            RegistrationError::Outbox(e) => e.to_error_code(),
            RegistrationError::TokenIssuer(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for IssueVerificationCodeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            IssueVerificationCodeError::Repository(e) => e.to_http_status(),
            IssueVerificationCodeError::Hash(e) => e.to_http_status(),
            IssueVerificationCodeError::Outbox(e) => e.to_http_status(),
            IssueVerificationCodeError::UnsupportedChannel => StatusCode::BAD_REQUEST,
            IssueVerificationCodeError::UserAlreadyVerified => StatusCode::CONFLICT,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            IssueVerificationCodeError::Repository(e) => e.to_user_message(),
            IssueVerificationCodeError::Hash(e) => e.to_user_message(),
            IssueVerificationCodeError::Outbox(e) => e.to_user_message(),
            IssueVerificationCodeError::UnsupportedChannel => {
                "Unsupported verification channel".to_string()
            }
            IssueVerificationCodeError::UserAlreadyVerified => "User already verified".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            IssueVerificationCodeError::Repository(e) => e.to_error_code(),
            IssueVerificationCodeError::Hash(e) => e.to_error_code(),
            IssueVerificationCodeError::Outbox(e) => e.to_error_code(),
            IssueVerificationCodeError::UnsupportedChannel => "VERIFICATION_CHANNEL_UNSUPPORTED",
            IssueVerificationCodeError::UserAlreadyVerified => "USER_ALREADY_VERIFIED",
        }
    }
}

impl ToHttpStatus for CreateServerError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateServerError::Repository(e) => e.to_http_status(),
            CreateServerError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            CreateServerError::Repository(e) => e.to_user_message(),
            CreateServerError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            CreateServerError::Repository(e) => e.to_error_code(),
            CreateServerError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for DeleteServerError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteServerError::ServerNotFound => StatusCode::NOT_FOUND,
            DeleteServerError::NotOwner => StatusCode::FORBIDDEN,
            DeleteServerError::Repository(e) => e.to_http_status(),
            DeleteServerError::Outbox(e) => e.to_http_status(),
            DeleteServerError::VoiceService(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteServerError::ServerNotFound => "Server not found".to_string(),
            DeleteServerError::NotOwner => "Only the server owner can delete it.".to_string(),
            DeleteServerError::Repository(e) => e.to_user_message(),
            DeleteServerError::Outbox(e) => e.to_user_message(),
            DeleteServerError::VoiceService(_) => {
                "Voice service is temporarily unavailable.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteServerError::ServerNotFound => "SERVER_NOT_FOUND",
            DeleteServerError::NotOwner => "SERVER_NOT_OWNER",
            DeleteServerError::Repository(e) => e.to_error_code(),
            DeleteServerError::Outbox(e) => e.to_error_code(),
            DeleteServerError::VoiceService(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for JoinVoiceChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinVoiceChannelError::ChannelNotFound | JoinVoiceChannelError::UserNotFound => {
                StatusCode::NOT_FOUND
            }
            JoinVoiceChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            JoinVoiceChannelError::InvalidPassword => StatusCode::UNAUTHORIZED,
            JoinVoiceChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            JoinVoiceChannelError::Repository(e) => e.to_http_status(),
            JoinVoiceChannelError::VoiceService(e) => e.to_http_status(),
            JoinVoiceChannelError::Hash(e) => e.to_http_status(),
            JoinVoiceChannelError::Domain(e) => e.to_http_status(),
            JoinVoiceChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            JoinVoiceChannelError::ChannelNotFound => "Channel not found".to_string(),
            JoinVoiceChannelError::UserNotFound => "User not found".to_string(),
            JoinVoiceChannelError::PasswordRequired => {
                "Password is required to join this channel".to_string()
            }
            JoinVoiceChannelError::InvalidPassword => "Invalid password".to_string(),
            JoinVoiceChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            JoinVoiceChannelError::Repository(e) => e.to_user_message(),
            JoinVoiceChannelError::VoiceService(e) => e.to_user_message(),
            JoinVoiceChannelError::Hash(e) => e.to_user_message(),
            JoinVoiceChannelError::Domain(e) => e.to_user_message(),
            JoinVoiceChannelError::Outbox(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            JoinVoiceChannelError::ChannelNotFound => "CHANNEL_NOT_FOUND",
            JoinVoiceChannelError::UserNotFound => "USER_NOT_FOUND",
            JoinVoiceChannelError::PasswordRequired => "CHANNEL_PASSWORD_REQUIRED",
            JoinVoiceChannelError::InvalidPassword => "INVALID_CHANNEL_PASSWORD",
            JoinVoiceChannelError::VoiceNotSupported => "VOICE_NOT_SUPPORTED",
            JoinVoiceChannelError::Repository(e) => e.to_error_code(),
            JoinVoiceChannelError::VoiceService(e) => e.to_error_code(),
            JoinVoiceChannelError::Hash(e) => e.to_error_code(),
            JoinVoiceChannelError::Domain(e) => e.to_error_code(),
            JoinVoiceChannelError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for ListServersByUserError {
    fn to_http_status(&self) -> StatusCode {
        let ListServersByUserError::Repository(e) = self;
        e.to_http_status()
    }

    fn to_user_message(&self) -> String {
        let ListServersByUserError::Repository(e) = self;
        e.to_user_message()
    }

    fn to_error_code(&self) -> &'static str {
        let ListServersByUserError::Repository(e) = self;
        e.to_error_code()
    }
}

impl ToHttpStatus for ListServerMembersError {
    fn to_http_status(&self) -> StatusCode {
        let ListServerMembersError::Repository(e) = self;
        e.to_http_status()
    }

    fn to_user_message(&self) -> String {
        let ListServerMembersError::Repository(e) = self;
        e.to_user_message()
    }

    fn to_error_code(&self) -> &'static str {
        let ListServerMembersError::Repository(e) = self;
        e.to_error_code()
    }
}

impl ToHttpStatus for ListServerChannelsError {
    fn to_http_status(&self) -> StatusCode {
        let ListServerChannelsError::Repository(e) = self;
        e.to_http_status()
    }

    fn to_user_message(&self) -> String {
        let ListServerChannelsError::Repository(e) = self;
        e.to_user_message()
    }

    fn to_error_code(&self) -> &'static str {
        let ListServerChannelsError::Repository(e) = self;
        e.to_error_code()
    }
}

#[derive(Debug)]
pub struct ApiError {
    message: String,
    status: StatusCode,
    code: &'static str,
    request_id: Option<String>,
    fields: Option<Vec<FieldError>>,
}

impl ApiError {
    pub fn new(message: String, status: StatusCode) -> Self {
        Self {
            message,
            status,
            code: "INTERNAL_ERROR",
            request_id: None,
            fields: None,
        }
    }

    fn from_error<E: ToHttpStatus + std::fmt::Debug>(error: E) -> Self {
        tracing::error!("API Error: {:?}", error);
        Self {
            message: error.to_user_message(),
            status: error.to_http_status(),
            code: error.to_error_code(),
            request_id: None,
            fields: None,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        let response = ErrorResponse {
            code: self.code,
            message: self.message.clone(),
            status: self.status.as_u16(),
            request_id: self.request_id.clone(),
            fields: self.fields.clone(),
        };
        let mut builder = HttpResponse::build(self.status);
        if let Some(ref rid) = self.request_id {
            if let Ok(value) = HeaderValue::from_str(rid) {
                builder.insert_header(("X-Request-Id", value));
            }
        }
        builder.json(response)
    }
}

impl From<CreateChannelError> for ApiError {
    fn from(error: CreateChannelError) -> Self {
        Self::from_error(error)
    }
}

impl From<DeleteChannelError> for ApiError {
    fn from(error: DeleteChannelError) -> Self {
        Self::from_error(error)
    }
}

impl From<MuteChannelError> for ApiError {
    fn from(error: MuteChannelError) -> Self {
        Self::from_error(error)
    }
}

impl From<UnmuteChannelError> for ApiError {
    fn from(error: UnmuteChannelError) -> Self {
        Self::from_error(error)
    }
}

impl From<LoginError> for ApiError {
    fn from(error: LoginError) -> Self {
        Self::from_error(error)
    }
}

impl From<RegistrationError> for ApiError {
    fn from(error: RegistrationError) -> Self {
        Self::from_error(error)
    }
}

impl From<IssueVerificationCodeError> for ApiError {
    fn from(error: IssueVerificationCodeError) -> Self {
        Self::from_error(error)
    }
}

impl From<ConfirmRegistrationError> for ApiError {
    fn from(error: ConfirmRegistrationError) -> Self {
        Self::from_error(error)
    }
}

impl From<RefreshTokensError> for ApiError {
    fn from(error: RefreshTokensError) -> Self {
        Self::from_error(error)
    }
}

impl From<LogoutError> for ApiError {
    fn from(error: LogoutError) -> Self {
        Self::from_error(error)
    }
}

impl From<HandleVoiceWebhookError> for ApiError {
    fn from(error: HandleVoiceWebhookError) -> Self {
        Self::from_error(error)
    }
}

impl From<CreateServerError> for ApiError {
    fn from(error: CreateServerError) -> Self {
        Self::from_error(error)
    }
}

impl From<DeleteServerError> for ApiError {
    fn from(error: DeleteServerError) -> Self {
        Self::from_error(error)
    }
}

impl From<PaginationError> for ApiError {
    fn from(error: PaginationError) -> Self {
        Self::from_error(error)
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(error: ValidationErrors) -> Self {
        tracing::error!("Validation error: {:?}", error);
        let fields = flatten_validation_errors(&error, "");
        Self {
            message: "Validation failed".to_string(),
            status: StatusCode::BAD_REQUEST,
            code: "VALIDATION_FAILED",
            request_id: None,
            fields: Some(fields),
        }
    }
}

fn flatten_validation_errors(errors: &ValidationErrors, prefix: &str) -> Vec<FieldError> {
    let mut out = Vec::new();
    for (field, kind) in errors.errors() {
        let path = if prefix.is_empty() {
            field.to_string()
        } else {
            format!("{prefix}.{field}")
        };
        match kind {
            ValidationErrorsKind::Field(errs) => {
                for e in errs {
                    let message = e
                        .message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| code_to_message(&e.code));
                    out.push(FieldError {
                        field: path.clone(),
                        code: e.code.to_string(),
                        message,
                    });
                }
            }
            ValidationErrorsKind::Struct(inner) => {
                out.extend(flatten_validation_errors(inner, &path));
            }
            ValidationErrorsKind::List(map) => {
                for (idx, inner) in map.iter() {
                    let item_path = format!("{path}.{idx}");
                    out.extend(flatten_validation_errors(inner, &item_path));
                }
            }
        }
    }
    out
}

fn code_to_message(code: &Cow<'static, str>) -> String {
    format!("validation failed: {code}")
}

impl From<JoinVoiceChannelError> for ApiError {
    fn from(error: JoinVoiceChannelError) -> Self {
        Self::from_error(error)
    }
}

impl From<ListServersByUserError> for ApiError {
    fn from(error: ListServersByUserError) -> Self {
        Self::from_error(error)
    }
}

impl From<ListServerMembersError> for ApiError {
    fn from(error: ListServerMembersError) -> Self {
        Self::from_error(error)
    }
}

impl From<ListServerChannelsError> for ApiError {
    fn from(error: ListServerChannelsError) -> Self {
        Self::from_error(error)
    }
}
