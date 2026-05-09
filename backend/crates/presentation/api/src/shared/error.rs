use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use application::ports::{
    hasher::HashError, mailer::MailerError, voice_service::VoiceServiceError,
};
use application::usecases::channel::join_channel::JoinChannelError;
use application::usecases::server::list_server_members::ListServerMembersError;
use application::usecases::server::list_server_channels::ListServerChannelsError;
use application::usecases::server::list_servers_by_user::ListServersByUserError;
use application::usecases::{
    auth::{login::LoginError, register::RegistrationError},
    channel::{create_channel::CreateChannelError, delete_channel::DeleteChannelError},
    server::{create_server::CreateServerError, delete_server::DeleteServerError},
};
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

impl ToHttpStatus for CreateChannelError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateChannelError::Repository(e) => e.to_http_status(),
            CreateChannelError::Hash(e) => e.to_http_status(),
            CreateChannelError::PasswordRequired => StatusCode::BAD_REQUEST,
            CreateChannelError::InvalidData => StatusCode::BAD_REQUEST,
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
        }
    }
}

impl ToHttpStatus for DeleteChannelError {
    fn to_http_status(&self) -> StatusCode {
        let DeleteChannelError::Repository(e) = self;
        e.to_http_status()
    }

    fn to_user_message(&self) -> String {
        let DeleteChannelError::Repository(e) = self;
        e.to_user_message()
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
            JoinChannelError::VoiceService(e) => e.to_http_status(),
            JoinChannelError::Repository(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            JoinChannelError::VoiceService(e) => e.to_user_message(),
            JoinChannelError::Repository(e) => e.to_user_message(),
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
