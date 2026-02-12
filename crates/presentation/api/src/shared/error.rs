use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use application::usecases::room::{
    create_room::CreateRoomError, generate_token::GenerateTokenError,
};
use serde::Serialize;
use validator::ValidationErrors;

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
    status: u16,
}

#[derive(Debug)]
pub struct ApiError {
    kind: ApiErrorKind,
}

#[derive(Debug)]
enum ApiErrorKind {
    CreateRoom(CreateRoomError),
    GenerateToken(GenerateTokenError),
    Validation(ValidationErrors),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ApiErrorKind::CreateRoom(e) => write!(f, "{}", e),
            ApiErrorKind::GenerateToken(e) => write!(f, "{}", e),
            ApiErrorKind::Validation(e) => write!(f, "{}", e),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match &self.kind {
            ApiErrorKind::CreateRoom(e) => match e {
                CreateRoomError::Domain(_) => StatusCode::BAD_REQUEST,
                CreateRoomError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
                CreateRoomError::Hash(_) => StatusCode::INTERNAL_SERVER_ERROR,
                CreateRoomError::PasswordRequired => StatusCode::BAD_REQUEST,
            },
            ApiErrorKind::GenerateToken(e) => match e {
                GenerateTokenError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
                GenerateTokenError::VoiceService(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            ApiErrorKind::Validation(_) => StatusCode::NOT_ACCEPTABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let response = ErrorResponse {
            message: self.to_string(),
            status: self.status_code().as_u16(),
        };
        HttpResponse::build(self.status_code()).json(response)
    }
}

impl From<CreateRoomError> for ApiError {
    fn from(value: CreateRoomError) -> Self {
        Self {
            kind: ApiErrorKind::CreateRoom(value),
        }
    }
}

impl From<GenerateTokenError> for ApiError {
    fn from(value: GenerateTokenError) -> Self {
        Self {
            kind: ApiErrorKind::GenerateToken(value),
        }
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(value: ValidationErrors) -> Self {
        Self {
            kind: ApiErrorKind::Validation(value),
        }
    }
}
