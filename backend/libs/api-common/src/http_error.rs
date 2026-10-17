use ports::{file_storage::FileStorageError, hasher::HashError, mailer::MailerError, voice_service::VoiceServiceError};

use crate::api_error::{StatusCode, ToHttpStatus};

#[macro_export]
macro_rules! http_error_boilerplate {
    () => {
        pub trait HttpMapping {
            fn to_http_status(&self) -> $crate::api_error::StatusCode;
            fn to_user_message(&self) -> String;
            fn to_error_code(&self) -> &'static str;
        }

        #[derive(Debug)]
        pub struct HttpError(pub $crate::api_error::ApiError);

        impl std::fmt::Display for HttpError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl actix_web::ResponseError for HttpError {
            fn status_code(&self) -> $crate::api_error::StatusCode {
                self.0.status_code()
            }

            fn error_response(&self) -> actix_web::HttpResponse {
                self.0.error_response()
            }
        }

        #[allow(dead_code)]
        impl HttpError {
            pub fn new(message: String, status: $crate::api_error::StatusCode, code: &'static str) -> Self {
                Self($crate::api_error::ApiError::new(message, status, code))
            }

            pub fn bad_request(message: String) -> Self {
                Self($crate::api_error::ApiError::bad_request(message))
            }

            pub fn payload_too_large() -> Self {
                Self($crate::api_error::ApiError::payload_too_large())
            }

            pub fn from_error<E: $crate::api_error::ToHttpStatus + std::fmt::Debug>(error: E) -> Self {
                Self($crate::api_error::ApiError::from_error(error))
            }
        }

        impl From<$crate::api_error::ApiError> for HttpError {
            fn from(error: $crate::api_error::ApiError) -> Self {
                Self(error)
            }
        }

        impl From<validator::ValidationErrors> for HttpError {
            fn from(error: validator::ValidationErrors) -> Self {
                Self($crate::api_error::ApiError::from(error))
            }
        }

        $crate::http_error_from_status!(
            ports::repository::RepositoryError,
            $crate::pagination::PaginationError,
            ports::voice_service::VoiceServiceError,
            ports::hasher::HashError,
            ports::mailer::MailerError,
            ports::file_storage::FileStorageError,
        );

        impl<E: HttpMapping + std::fmt::Debug> From<E> for HttpError {
            fn from(error: E) -> Self {
                tracing::error!("API Error: {:?}", error);
                Self($crate::api_error::ApiError::new(
                    error.to_user_message(),
                    error.to_http_status(),
                    error.to_error_code(),
                ))
            }
        }
    };
}

#[macro_export]
macro_rules! http_error_from_status {
    ($($err:ty),* $(,)?) => {
        $(
            impl From<$err> for HttpError {
                fn from(error: $err) -> Self {
                    Self($crate::api_error::ApiError::from_error(error))
                }
            }
        )*
    };
}

#[macro_export]
macro_rules! impl_http_mapping_delegate {
    ($err:ident, $variant:ident) => {
        impl HttpMapping for $err {
            fn to_http_status(&self) -> $crate::api_error::StatusCode {
                let $err::$variant(e) = self;
                $crate::api_error::ToHttpStatus::to_http_status(e)
            }

            fn to_user_message(&self) -> String {
                let $err::$variant(e) = self;
                $crate::api_error::ToHttpStatus::to_user_message(e)
            }

            fn to_error_code(&self) -> &'static str {
                let $err::$variant(e) = self;
                $crate::api_error::ToHttpStatus::to_error_code(e)
            }
        }
    };
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
            VoiceServiceError::ParticipantRemoval(_) => "VOICE_PARTICIPANT_REMOVAL_FAILED",
            VoiceServiceError::ParticipantUpdate(_) => "VOICE_PARTICIPANT_UPDATE_FAILED",
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
            FileStorageError::Internal(_) => "File storage is temporarily unavailable.".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            FileStorageError::NotFound => "ATTACHMENT_NOT_FOUND",
            FileStorageError::Internal(_) => "STORAGE_UNAVAILABLE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_adapter_errors_map_to_the_same_http_answer_in_every_service() {
        assert_eq!(FileStorageError::NotFound.to_http_status(), StatusCode::NOT_FOUND);
        assert_eq!(FileStorageError::NotFound.to_error_code(), "ATTACHMENT_NOT_FOUND");
        assert_eq!(MailerError::Sending("smtp".into()).to_http_status(), StatusCode::BAD_GATEWAY);
        assert_eq!(VoiceServiceError::TokenCreation("x".into()).to_error_code(), "VOICE_TOKEN_CREATION_FAILED");
    }
}
