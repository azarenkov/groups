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

impl From<ports::repository::RepositoryError> for HttpError {
    fn from(error: ports::repository::RepositoryError) -> Self {
        Self(ApiError::from(error))
    }
}

impl From<api_common::pagination::PaginationError> for HttpError {
    fn from(error: api_common::pagination::PaginationError) -> Self {
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

use media::ports::image_moderation::ImageModerationError;
use media::usecases::attachment::delete_attachment::DeleteAttachmentError;
use media::usecases::attachment::upload_attachment::UploadAttachmentError;
use media::usecases::community::get_logo::GetCommunityLogoError;
use media::usecases::community::update_logo::UpdateCommunityLogoError;
use media::usecases::user::delete_avatar::DeleteUserAvatarError;
use media::usecases::user::get_avatar::GetUserAvatarError;
use media::usecases::user::update_avatar::UpdateUserAvatarError;

impl HttpMapping for UploadAttachmentError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UploadAttachmentError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            UploadAttachmentError::Empty => StatusCode::BAD_REQUEST,
            UploadAttachmentError::TypeNotAllowed => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            UploadAttachmentError::QuotaExceeded => StatusCode::TOO_MANY_REQUESTS,
            UploadAttachmentError::InvalidVoice => StatusCode::BAD_REQUEST,
            UploadAttachmentError::VoiceTooLong => StatusCode::PAYLOAD_TOO_LARGE,
            UploadAttachmentError::Storage(e) => e.to_http_status(),
            UploadAttachmentError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            UploadAttachmentError::TooLarge => "File is too large for your plan.".to_string(),
            UploadAttachmentError::Empty => "File is empty.".to_string(),
            UploadAttachmentError::TypeNotAllowed => {
                "This file type is not allowed.".to_string()
            }
            UploadAttachmentError::QuotaExceeded => {
                "Too many pending uploads. Send or remove existing files first.".to_string()
            }
            UploadAttachmentError::InvalidVoice => "This voice message can't be sent.".to_string(),
            UploadAttachmentError::VoiceTooLong => {
                "This voice message is too long for your plan.".to_string()
            }
            UploadAttachmentError::Storage(e) => e.to_user_message(),
            UploadAttachmentError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            UploadAttachmentError::TooLarge => "ATTACHMENT_TOO_LARGE",
            UploadAttachmentError::Empty => "ATTACHMENT_EMPTY",
            UploadAttachmentError::TypeNotAllowed => "ATTACHMENT_TYPE_NOT_ALLOWED",
            UploadAttachmentError::QuotaExceeded => "ATTACHMENT_QUOTA_EXCEEDED",
            UploadAttachmentError::InvalidVoice => "VOICE_INVALID",
            UploadAttachmentError::VoiceTooLong => "VOICE_TOO_LONG",
            UploadAttachmentError::Storage(e) => e.to_error_code(),
            UploadAttachmentError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for ImageModerationError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ImageModerationError::Rejected => StatusCode::UNPROCESSABLE_ENTITY,
            ImageModerationError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ImageModerationError::Rejected => {
                "Image was rejected by content moderation.".to_string()
            }
            ImageModerationError::Unavailable(_) => {
                "Content moderation is temporarily unavailable. Try again later.".to_string()
            }
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ImageModerationError::Rejected => "IMAGE_CONTENT_REJECTED",
            ImageModerationError::Unavailable(_) => "MODERATION_UNAVAILABLE",
        }
    }
}

impl HttpMapping for media::usecases::user::banner::BannerError {
    fn to_http_status(&self) -> StatusCode {
        use media::usecases::user::banner::BannerError as E;
        match self {
            E::ProRequired => StatusCode::FORBIDDEN,
            E::NotFound => StatusCode::NOT_FOUND,
            E::Repository(e) => e.to_http_status(),
            E::Storage(e) => e.to_http_status(),
            E::Moderation(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        use media::usecases::user::banner::BannerError as E;
        match self {
            E::ProRequired => "This needs a Rift Pro subscription.".to_string(),
            E::NotFound => "Banner not found".to_string(),
            E::Repository(e) => e.to_user_message(),
            E::Storage(e) => e.to_user_message(),
            E::Moderation(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        use media::usecases::user::banner::BannerError as E;
        match self {
            E::ProRequired => "PRO_REQUIRED",
            E::NotFound => "BANNER_NOT_FOUND",
            E::Repository(e) => e.to_error_code(),
            E::Storage(e) => e.to_error_code(),
            E::Moderation(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for UpdateUserAvatarError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UpdateUserAvatarError::Repository(e) => e.to_http_status(),
            UpdateUserAvatarError::Storage(e) => e.to_http_status(),
            UpdateUserAvatarError::Moderation(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            UpdateUserAvatarError::Repository(e) => e.to_user_message(),
            UpdateUserAvatarError::Storage(e) => e.to_user_message(),
            UpdateUserAvatarError::Moderation(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            UpdateUserAvatarError::Repository(e) => e.to_error_code(),
            UpdateUserAvatarError::Storage(e) => e.to_error_code(),
            UpdateUserAvatarError::Moderation(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for DeleteUserAvatarError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteUserAvatarError::Repository(e) => e.to_http_status(),
            DeleteUserAvatarError::Storage(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DeleteUserAvatarError::Repository(e) => e.to_user_message(),
            DeleteUserAvatarError::Storage(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteUserAvatarError::Repository(e) => e.to_error_code(),
            DeleteUserAvatarError::Storage(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for GetUserAvatarError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetUserAvatarError::NotFound => StatusCode::NOT_FOUND,
            GetUserAvatarError::Repository(e) => e.to_http_status(),
            GetUserAvatarError::Storage(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetUserAvatarError::NotFound => "Avatar not found.".to_string(),
            GetUserAvatarError::Repository(e) => e.to_user_message(),
            GetUserAvatarError::Storage(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetUserAvatarError::NotFound => "AVATAR_NOT_FOUND",
            GetUserAvatarError::Repository(e) => e.to_error_code(),
            GetUserAvatarError::Storage(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for UpdateCommunityLogoError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UpdateCommunityLogoError::CommunityNotFound => StatusCode::NOT_FOUND,
            UpdateCommunityLogoError::NotOwner => StatusCode::FORBIDDEN,
            UpdateCommunityLogoError::OwnerLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
            UpdateCommunityLogoError::Repository(e) => e.to_http_status(),
            UpdateCommunityLogoError::Storage(e) => e.to_http_status(),
            UpdateCommunityLogoError::Moderation(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            UpdateCommunityLogoError::CommunityNotFound => "Community not found.".to_string(),
            UpdateCommunityLogoError::NotOwner => {
                "Only the community owner can change the logo.".to_string()
            }
            UpdateCommunityLogoError::OwnerLookup(_) => {
                "Community ownership is temporarily unavailable. Try again later.".to_string()
            }
            UpdateCommunityLogoError::Repository(e) => e.to_user_message(),
            UpdateCommunityLogoError::Storage(e) => e.to_user_message(),
            UpdateCommunityLogoError::Moderation(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            UpdateCommunityLogoError::CommunityNotFound => "COMMUNITY_NOT_FOUND",
            UpdateCommunityLogoError::NotOwner => "COMMUNITY_NOT_OWNER",
            UpdateCommunityLogoError::OwnerLookup(_) => "OWNER_LOOKUP_UNAVAILABLE",
            UpdateCommunityLogoError::Repository(e) => e.to_error_code(),
            UpdateCommunityLogoError::Storage(e) => e.to_error_code(),
            UpdateCommunityLogoError::Moderation(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for GetCommunityLogoError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetCommunityLogoError::NotFound => StatusCode::NOT_FOUND,
            GetCommunityLogoError::Repository(e) => e.to_http_status(),
            GetCommunityLogoError::Storage(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetCommunityLogoError::NotFound => "Logo not found.".to_string(),
            GetCommunityLogoError::Repository(e) => e.to_user_message(),
            GetCommunityLogoError::Storage(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetCommunityLogoError::NotFound => "LOGO_NOT_FOUND",
            GetCommunityLogoError::Repository(e) => e.to_error_code(),
            GetCommunityLogoError::Storage(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for DeleteAttachmentError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteAttachmentError::NotFound => StatusCode::NOT_FOUND,
            DeleteAttachmentError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DeleteAttachmentError::NotFound => "Attachment not found.".to_string(),
            DeleteAttachmentError::Repository(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteAttachmentError::NotFound => "ATTACHMENT_NOT_FOUND",
            DeleteAttachmentError::Repository(e) => e.to_error_code(),
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
