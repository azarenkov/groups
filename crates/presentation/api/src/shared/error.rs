use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use application::usecases::{
    auth::{login::LoginError, register::RegistrationError},
    room::{
        create_room::CreateRoomError, delete_room::DeleteRoomError,
        generate_token::GenerateTokenError,
    },
    server::{create_server::CreateServerError, delete_server::DeleteServerError},
};
use application::ports::{hasher::HashError, voice_service::VoiceServiceError};
use domain::shared::repository::error::RepositoryError;
use serde::Serialize;
use validator::ValidationErrors;

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
    status: u16,
}

// Trait definition for converting errors to HTTP status codes and user messages
pub trait ToHttpStatus {
    fn to_http_status(&self) -> StatusCode;
    fn to_user_message(&self) -> String;
}

// Implement for RepositoryError (domain layer)
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

// Implement for HashError
impl ToHttpStatus for HashError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Authentication error".to_string()
    }
}

// Implement for VoiceServiceError
impl ToHttpStatus for VoiceServiceError {
    fn to_http_status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn to_user_message(&self) -> String {
        "Voice service error".to_string()
    }
}

// Implement for CreateRoomError
impl ToHttpStatus for CreateRoomError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            CreateRoomError::Repository(e) => e.to_http_status(),
            CreateRoomError::Hash(e) => e.to_http_status(),
            CreateRoomError::PasswordRequired => StatusCode::BAD_REQUEST,
            CreateRoomError::InvalidData => StatusCode::BAD_REQUEST,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            CreateRoomError::Repository(e) => e.to_user_message(),
            CreateRoomError::Hash(e) => e.to_user_message(),
            CreateRoomError::PasswordRequired => "Password is required for private rooms".to_string(),
            CreateRoomError::InvalidData => "Invalid room data provided".to_string(),
        }
    }
}

// Implement for GenerateTokenError
impl ToHttpStatus for GenerateTokenError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GenerateTokenError::Repository(e) => e.to_http_status(),
            GenerateTokenError::VoiceService(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            GenerateTokenError::Repository(e) => e.to_user_message(),
            GenerateTokenError::VoiceService(e) => e.to_user_message(),
        }
    }
}

// Implement for DeleteRoomError
impl ToHttpStatus for DeleteRoomError {
    fn to_http_status(&self) -> StatusCode {
        let DeleteRoomError::Repository(e) = self;
        e.to_http_status()
    }

    fn to_user_message(&self) -> String {
        let DeleteRoomError::Repository(e) = self;
        e.to_user_message()
    }
}

// Implement for LoginError
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

// Implement for RegistrationError
impl ToHttpStatus for RegistrationError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RegistrationError::Repository(e) => e.to_http_status(),
            RegistrationError::Hash(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RegistrationError::Repository(e) => e.to_user_message(),
            RegistrationError::Hash(e) => e.to_user_message(),
        }
    }
}

// Implement for CreateServerError
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

// Implement for DeleteServerError
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

// Simplified ApiError struct
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

// From implementations using the trait
impl From<CreateRoomError> for ApiError {
    fn from(error: CreateRoomError) -> Self {
        Self::from_error(error)
    }
}

impl From<GenerateTokenError> for ApiError {
    fn from(error: GenerateTokenError) -> Self {
        Self::from_error(error)
    }
}

impl From<DeleteRoomError> for ApiError {
    fn from(error: DeleteRoomError) -> Self {
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

// Special handling for ValidationErrors
impl From<ValidationErrors> for ApiError {
    fn from(error: ValidationErrors) -> Self {
        tracing::error!("Validation error: {:?}", error);
        Self {
            message: error.to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }
}
