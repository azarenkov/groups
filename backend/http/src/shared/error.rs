use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use application::ports::{
    hasher::HashError, mailer::MailerError, voice_service::VoiceServiceError,
};
use application::usecases::channel::join_channel::JoinChannelError;
use application::usecases::server::list_server_channels::ListServerChannelsError;
use application::usecases::server::list_server_members::ListServerMembersError;
use application::usecases::server::list_servers_by_user::ListServersByUserError;
use application::usecases::{
    auth::{login::LoginError, register::RegistrationError},
    channel::{
        create_channel::CreateChannelError, delete_channel::DeleteChannelError,
        leave_channel::LeaveChannelError, mute_channel::MuteChannelError,
        unmute_channel::UnmuteChannelError,
    },
    server::{create_server::CreateServerError, delete_server::DeleteServerError},
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

impl ToHttpStatus for LeaveChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LeaveChannelError::ChannelNotFound => StatusCode::NOT_FOUND,
            LeaveChannelError::Repository(e) => e.to_http_status(),
            LeaveChannelError::Domain(e) => e.to_http_status(),
            LeaveChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LeaveChannelError::ChannelNotFound => "Channel not found".to_string(),
            LeaveChannelError::Repository(e) => e.to_user_message(),
            LeaveChannelError::Domain(e) => e.to_user_message(),
            LeaveChannelError::Outbox(e) => e.to_user_message(),
        }
    }
}

impl ToHttpStatus for MuteChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            MuteChannelError::ChannelNotFound | MuteChannelError::TargetUserNotFound => {
                StatusCode::NOT_FOUND
            }
            MuteChannelError::Repository(e) => e.to_http_status(),
            MuteChannelError::Domain(e) => e.to_http_status(),
            MuteChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            MuteChannelError::ChannelNotFound => "Channel not found".to_string(),
            MuteChannelError::TargetUserNotFound => "Target user not found".to_string(),
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
            UnmuteChannelError::Repository(e) => e.to_http_status(),
            UnmuteChannelError::Domain(e) => e.to_http_status(),
            UnmuteChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            UnmuteChannelError::ChannelNotFound => "Channel not found".to_string(),
            UnmuteChannelError::TargetUserNotFound => "Target user not found".to_string(),
            UnmuteChannelError::Repository(e) => e.to_user_message(),
            UnmuteChannelError::Domain(e) => e.to_user_message(),
            UnmuteChannelError::Outbox(e) => e.to_user_message(),
        }
    }
}

impl ToHttpStatus for LoginError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LoginError::Repository(e) => e.to_http_status(),
            LoginError::Hash(e) => e.to_http_status(),
            LoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            LoginError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LoginError::Repository(e) => e.to_user_message(),
            LoginError::Hash(e) => e.to_user_message(),
            LoginError::InvalidCredentials => "Invalid credentials".to_string(),
            LoginError::Internal => "Internal server error".to_string(),
        }
    }
}

impl ToHttpStatus for RegistrationError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RegistrationError::Repository(e) => e.to_http_status(),
            RegistrationError::Hash(e) => e.to_http_status(),
            RegistrationError::Mailer(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RegistrationError::Repository(e) => e.to_user_message(),
            RegistrationError::Hash(e) => e.to_user_message(),
            RegistrationError::Mailer(e) => e.to_user_message(),
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

impl ToHttpStatus for JoinChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            JoinChannelError::ChannelNotFound | JoinChannelError::UserNotFound => {
                StatusCode::NOT_FOUND
            }
            JoinChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            JoinChannelError::InvalidPassword => StatusCode::UNAUTHORIZED,
            JoinChannelError::Repository(e) => e.to_http_status(),
            JoinChannelError::VoiceService(e) => e.to_http_status(),
            JoinChannelError::Hash(e) => e.to_http_status(),
            JoinChannelError::Domain(e) => e.to_http_status(),
            JoinChannelError::Outbox(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            JoinChannelError::ChannelNotFound => "Channel not found".to_string(),
            JoinChannelError::UserNotFound => "User not found".to_string(),
            JoinChannelError::PasswordRequired => {
                "Password is required to join this channel".to_string()
            }
            JoinChannelError::InvalidPassword => "Invalid password".to_string(),
            JoinChannelError::Repository(e) => e.to_user_message(),
            JoinChannelError::VoiceService(e) => e.to_user_message(),
            JoinChannelError::Hash(e) => e.to_user_message(),
            JoinChannelError::Domain(e) => e.to_user_message(),
            JoinChannelError::Outbox(e) => e.to_user_message(),
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

impl From<LeaveChannelError> for ApiError {
    fn from(error: LeaveChannelError) -> Self {
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

impl From<JoinChannelError> for ApiError {
    fn from(error: JoinChannelError) -> Self {
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
