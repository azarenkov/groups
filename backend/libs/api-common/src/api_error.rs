use actix_web::{
    HttpResponse, ResponseError,
    http::header::HeaderValue,
};

pub use actix_web::http::StatusCode;
use serde::Serialize;
use validator::{ValidationErrors, ValidationErrorsKind};

use crate::middleware::request_id::{self, REQUEST_ID_HEADER};

use crate::pagination::PaginationError;
use ports::{repository::RepositoryError};

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

#[macro_export]
macro_rules! impl_to_http_status_delegate {
    ($err:ident, $variant:ident) => {
        impl $crate::api_error::ToHttpStatus for $err {
            fn to_http_status(&self) -> $crate::api_error::StatusCode {
                let $err::$variant(e) = self;
                e.to_http_status()
            }

            fn to_user_message(&self) -> String {
                let $err::$variant(e) = self;
                e.to_user_message()
            }

            fn to_error_code(&self) -> &'static str {
                let $err::$variant(e) = self;
                e.to_error_code()
            }
        }
    };
}

#[derive(Debug)]
pub struct ApiError {
    message: String,
    status: StatusCode,
    code: &'static str,
    fields: Option<Vec<FieldError>>,
}

impl ApiError {
    pub fn new(message: String, status: StatusCode, code: &'static str) -> Self {
        Self {
            message,
            status,
            code,
            fields: None,
        }
    }

    pub fn bad_request(message: String) -> Self {
        Self {
            message,
            status: StatusCode::BAD_REQUEST,
            code: "BAD_REQUEST",
            fields: None,
        }
    }

    pub fn payload_too_large() -> Self {
        Self {
            message: "File is too large.".to_string(),
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "ATTACHMENT_TOO_LARGE",
            fields: None,
        }
    }

    pub fn from_error<E: ToHttpStatus + std::fmt::Debug>(error: E) -> Self {
        tracing::error!("API Error: {:?}", error);
        Self {
            message: error.to_user_message(),
            status: error.to_http_status(),
            code: error.to_error_code(),
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
            fields: self.fields.clone(),
        };
        let mut builder = HttpResponse::build(self.status);
        if let Some(value) = request_id::current().and_then(|r| HeaderValue::from_str(&r).ok()) {
            builder.insert_header((REQUEST_ID_HEADER, value));
        }
        builder.json(response)
    }
}

#[macro_export]
macro_rules! impl_from_for_api_error {
    ($($err:ty),* $(,)?) => {
        $(
            impl From<$err> for $crate::api_error::ApiError {
                fn from(error: $err) -> Self {
                    $crate::api_error::ApiError::from_error(error)
                }
            }
        )*
    };
}

impl From<ValidationErrors> for ApiError {
    fn from(error: ValidationErrors) -> Self {
        tracing::error!("Validation error: {:?}", error);
        let fields = flatten_validation_errors(&error, "");
        Self {
            message: "Validation failed".to_string(),
            status: StatusCode::BAD_REQUEST,
            code: "VALIDATION_FAILED",
            fields: Some(fields),
        }
    }
}

crate::impl_from_for_api_error!(
    PaginationError,
    RepositoryError,
);

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

fn code_to_message(code: &str) -> String {
    format!("validation failed: {code}")
}
