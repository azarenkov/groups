use actix_web::{HttpResponse, ResponseError, http::StatusCode};
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
        confirm_registration::ConfirmRegistrationError, login::LoginError, logout::LogoutError,
        refresh_tokens::RefreshTokensError, register::RegistrationError,
    },
    channel::{
        abort_voice_connection::AbortVoiceConnectionError, create_channel::CreateChannelError,
        delete_channel::DeleteChannelError, leave_voice_channel::LeaveVoiceChannelError,
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
use validator::ValidationErrors;

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
    status: u16,
}

pub trait ToHttpStatus {
    fn to_http_status(&self) -> StatusCode;
    fn to_user_message(&self) -> String;
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
}

impl ToHttpStatus for VoiceServiceError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Voice service error".to_string()
    }
}

impl ToHttpStatus for HashError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Authentication error".to_string()
    }
}

impl ToHttpStatus for MailerError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::BAD_GATEWAY
    }

    fn to_user_message(&self) -> String {
        "Could not send email. Please try again in a moment.".to_string()
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
}

impl ToHttpStatus for OutboxError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Internal server error".to_string()
    }
}

impl ToHttpStatus for PaginationError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }

    fn to_user_message(&self) -> String {
        self.to_string()
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
}

impl ToHttpStatus for DeleteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteChannelError::ChannelNotFound => StatusCode::NOT_FOUND,
            DeleteChannelError::Repository(e) => e.to_http_status(),
            DeleteChannelError::Domain(e) => e.to_http_status(),
            DeleteChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            DeleteChannelError::ChannelNotFound => "Channel not found".to_string(),
            DeleteChannelError::Repository(e) => e.to_user_message(),
            DeleteChannelError::Domain(e) => e.to_user_message(),
            DeleteChannelError::Outbox(e) => e.to_user_message(),
        }
    }
}

impl ToHttpStatus for LeaveVoiceChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LeaveVoiceChannelError::ChannelNotFound => StatusCode::NOT_FOUND,
            LeaveVoiceChannelError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            LeaveVoiceChannelError::Repository(e) => e.to_http_status(),
            LeaveVoiceChannelError::Domain(e) => e.to_http_status(),
            LeaveVoiceChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LeaveVoiceChannelError::ChannelNotFound => "Channel not found".to_string(),
            LeaveVoiceChannelError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            LeaveVoiceChannelError::Repository(e) => e.to_user_message(),
            LeaveVoiceChannelError::Domain(e) => e.to_user_message(),
            LeaveVoiceChannelError::Outbox(e) => e.to_user_message(),
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
}

impl ToHttpStatus for AbortVoiceConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            AbortVoiceConnectionError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            AbortVoiceConnectionError::Repository(e) => e.to_http_status(),
            AbortVoiceConnectionError::Domain(e) => e.to_http_status(),
            AbortVoiceConnectionError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            AbortVoiceConnectionError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            AbortVoiceConnectionError::Repository(e) => e.to_user_message(),
            AbortVoiceConnectionError::Domain(e) => e.to_user_message(),
            AbortVoiceConnectionError::Outbox(e) => e.to_user_message(),
        }
    }
}

impl ToHttpStatus for LeaveVoiceConnectionError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LeaveVoiceConnectionError::VoiceNotSupported => StatusCode::BAD_REQUEST,
            LeaveVoiceConnectionError::Repository(e) => e.to_http_status(),
            LeaveVoiceConnectionError::Domain(e) => e.to_http_status(),
            LeaveVoiceConnectionError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LeaveVoiceConnectionError::VoiceNotSupported => {
                "Voice is not supported for this channel".to_string()
            }
            LeaveVoiceConnectionError::Repository(e) => e.to_user_message(),
            LeaveVoiceConnectionError::Domain(e) => e.to_user_message(),
            LeaveVoiceConnectionError::Outbox(e) => e.to_user_message(),
        }
    }
}

impl ToHttpStatus for HandleVoiceWebhookError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_http_status(),
            HandleVoiceWebhookError::Repository(e) => e.to_http_status(),
            HandleVoiceWebhookError::AbortVoiceConnection(e) => e.to_http_status(),
            HandleVoiceWebhookError::LeaveVoiceConnection(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            HandleVoiceWebhookError::Verifier(e) => e.to_user_message(),
            HandleVoiceWebhookError::Repository(e) => e.to_user_message(),
            HandleVoiceWebhookError::AbortVoiceConnection(e) => e.to_user_message(),
            HandleVoiceWebhookError::LeaveVoiceConnection(e) => e.to_user_message(),
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
}

impl ToHttpStatus for LoginError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LoginError::Repository(e) => e.to_http_status(),
            LoginError::Hash(e) => e.to_http_status(),
            LoginError::TokenIssuer(e) => e.to_http_status(),
            LoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LoginError::Repository(e) => e.to_user_message(),
            LoginError::Hash(e) => e.to_user_message(),
            LoginError::TokenIssuer(e) => e.to_user_message(),
            LoginError::InvalidCredentials => "Invalid credentials".to_string(),
        }
    }
}

impl ToHttpStatus for RefreshTokensError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RefreshTokensError::Repository(e) => e.to_http_status(),
            RefreshTokensError::TokenIssuer(e) => e.to_http_status(),
            RefreshTokensError::RefreshTokenNotFound => StatusCode::NOT_FOUND,
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
        }
    }
}

impl ToHttpStatus for LogoutError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LogoutError::Repository(e) => e.to_http_status(),
            LogoutError::TokenIssuer(e) => e.to_http_status(),
            LogoutError::RefreshTokenNotFound => StatusCode::NOT_FOUND,
            LogoutError::UserMismatch => StatusCode::UNAUTHORIZED,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LogoutError::Repository(e) => e.to_user_message(),
            LogoutError::TokenIssuer(e) => e.to_user_message(),
            LogoutError::RefreshTokenNotFound => "Refresh token not found".to_string(),
            LogoutError::UserMismatch => "Refresh token user mismatch".to_string(),
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
}

impl ToHttpStatus for CreateServerError {
    fn to_http_status(&self) -> StatusCode {
        let CreateServerError::Repository(e) = self;
        e.to_http_status()
    }

    fn to_user_message(&self) -> String {
        let CreateServerError::Repository(e) = self;
        e.to_user_message()
    }
}

impl ToHttpStatus for DeleteServerError {
    fn to_http_status(&self) -> StatusCode {
        let DeleteServerError::Repository(e) = self;
        e.to_http_status()
    }

    fn to_user_message(&self) -> String {
        let DeleteServerError::Repository(e) = self;
        e.to_user_message()
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
}

#[derive(Debug)]
pub struct ApiError {
    message: String,
    status: StatusCode,
}

impl ApiError {
    pub fn new(message: String, status: StatusCode) -> Self {
        Self { message, status }
    }

    fn from_error<E: ToHttpStatus + std::fmt::Debug>(error: E) -> Self {
        tracing::error!("API Error: {:?}", error);
        Self {
            message: error.to_user_message(),
            status: error.to_http_status(),
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
            message: self.message.clone(),
            status: self.status.as_u16(),
        };
        HttpResponse::build(self.status).json(response)
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

impl From<LeaveVoiceChannelError> for ApiError {
    fn from(error: LeaveVoiceChannelError) -> Self {
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
        Self {
            message: error.to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }
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
