use shared::api_error::{StatusCode, ToHttpStatus};
use crate::ports::image_moderation::ImageModerationError;
use crate::usecases::attachment::delete_attachment::DeleteAttachmentError;
use crate::usecases::attachment::upload_attachment::UploadAttachmentError;
use crate::usecases::server::get_logo::GetServerLogoError;
use crate::usecases::server::update_logo::UpdateServerLogoError;
use crate::usecases::user::delete_avatar::DeleteUserAvatarError;
use crate::usecases::user::get_avatar::GetUserAvatarError;
use crate::usecases::user::update_avatar::UpdateUserAvatarError;

impl ToHttpStatus for UploadAttachmentError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UploadAttachmentError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            UploadAttachmentError::Empty => StatusCode::BAD_REQUEST,
            UploadAttachmentError::TypeNotAllowed => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            UploadAttachmentError::QuotaExceeded => StatusCode::TOO_MANY_REQUESTS,
            UploadAttachmentError::Storage(e) => e.to_http_status(),
            UploadAttachmentError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            UploadAttachmentError::TooLarge => "File is too large (max 15 MB).".to_string(),
            UploadAttachmentError::Empty => "File is empty.".to_string(),
            UploadAttachmentError::TypeNotAllowed => {
                "This file type is not allowed.".to_string()
            }
            UploadAttachmentError::QuotaExceeded => {
                "Too many pending uploads. Send or remove existing files first.".to_string()
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
            UploadAttachmentError::Storage(e) => e.to_error_code(),
            UploadAttachmentError::Repository(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for ImageModerationError {
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

impl ToHttpStatus for UpdateUserAvatarError {
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

impl ToHttpStatus for DeleteUserAvatarError {
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

impl ToHttpStatus for GetUserAvatarError {
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

impl ToHttpStatus for UpdateServerLogoError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UpdateServerLogoError::ServerNotFound => StatusCode::NOT_FOUND,
            UpdateServerLogoError::NotOwner => StatusCode::FORBIDDEN,
            UpdateServerLogoError::OwnerLookup(_) => StatusCode::SERVICE_UNAVAILABLE,
            UpdateServerLogoError::Repository(e) => e.to_http_status(),
            UpdateServerLogoError::Storage(e) => e.to_http_status(),
            UpdateServerLogoError::Moderation(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            UpdateServerLogoError::ServerNotFound => "Server not found.".to_string(),
            UpdateServerLogoError::NotOwner => {
                "Only the server owner can change the logo.".to_string()
            }
            UpdateServerLogoError::OwnerLookup(_) => {
                "Server ownership is temporarily unavailable. Try again later.".to_string()
            }
            UpdateServerLogoError::Repository(e) => e.to_user_message(),
            UpdateServerLogoError::Storage(e) => e.to_user_message(),
            UpdateServerLogoError::Moderation(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            UpdateServerLogoError::ServerNotFound => "SERVER_NOT_FOUND",
            UpdateServerLogoError::NotOwner => "SERVER_NOT_OWNER",
            UpdateServerLogoError::OwnerLookup(_) => "OWNER_LOOKUP_UNAVAILABLE",
            UpdateServerLogoError::Repository(e) => e.to_error_code(),
            UpdateServerLogoError::Storage(e) => e.to_error_code(),
            UpdateServerLogoError::Moderation(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for GetServerLogoError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetServerLogoError::NotFound => StatusCode::NOT_FOUND,
            GetServerLogoError::Repository(e) => e.to_http_status(),
            GetServerLogoError::Storage(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            GetServerLogoError::NotFound => "Logo not found.".to_string(),
            GetServerLogoError::Repository(e) => e.to_user_message(),
            GetServerLogoError::Storage(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            GetServerLogoError::NotFound => "LOGO_NOT_FOUND",
            GetServerLogoError::Repository(e) => e.to_error_code(),
            GetServerLogoError::Storage(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for DeleteAttachmentError {
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

shared::impl_from_for_api_error!(
    DeleteAttachmentError,
    DeleteUserAvatarError,
    GetServerLogoError,
    GetUserAvatarError,
    UpdateServerLogoError,
    UpdateUserAvatarError,
    UploadAttachmentError,
);
