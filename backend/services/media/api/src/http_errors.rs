use api_common::api_error::{StatusCode, ToHttpStatus};

api_common::http_error_boilerplate!();

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
            UploadAttachmentError::InvalidVideoNote => StatusCode::BAD_REQUEST,
            UploadAttachmentError::VideoNoteTooLong => StatusCode::PAYLOAD_TOO_LARGE,
            UploadAttachmentError::Storage(e) => e.to_http_status(),
            UploadAttachmentError::Repository(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            UploadAttachmentError::TooLarge => "File is too large.".to_string(),
            UploadAttachmentError::Empty => "File is empty.".to_string(),
            UploadAttachmentError::TypeNotAllowed => {
                "This file type is not allowed.".to_string()
            }
            UploadAttachmentError::QuotaExceeded => {
                "Too many pending uploads. Send or remove existing files first.".to_string()
            }
            UploadAttachmentError::InvalidVoice => "This voice message can't be sent.".to_string(),
            UploadAttachmentError::VoiceTooLong => {
                "This voice message is too long.".to_string()
            }
            UploadAttachmentError::InvalidVideoNote => "This video message can't be sent.".to_string(),
            UploadAttachmentError::VideoNoteTooLong => "Video messages can be up to one minute long.".to_string(),
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
            UploadAttachmentError::InvalidVideoNote => "VIDEO_NOTE_INVALID",
            UploadAttachmentError::VideoNoteTooLong => "VIDEO_NOTE_TOO_LONG",
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
            E::NotFound => StatusCode::NOT_FOUND,
            E::Repository(e) => e.to_http_status(),
            E::Storage(e) => e.to_http_status(),
            E::Moderation(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        use media::usecases::user::banner::BannerError as E;
        match self {
            E::NotFound => "Banner not found".to_string(),
            E::Repository(e) => e.to_user_message(),
            E::Storage(e) => e.to_user_message(),
            E::Moderation(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        use media::usecases::user::banner::BannerError as E;
        match self {
            E::NotFound => "BANNER_NOT_FOUND",
            E::Repository(e) => e.to_error_code(),
            E::Storage(e) => e.to_error_code(),
            E::Moderation(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for media::usecases::sticker::StickerError {
    fn to_http_status(&self) -> StatusCode {
        use media::usecases::sticker::StickerError as E;
        match self {
            E::InvalidName | E::InvalidSlug | E::InvalidEmoji => StatusCode::BAD_REQUEST,
            E::Taken => StatusCode::CONFLICT,
            E::PackLimit => StatusCode::FORBIDDEN,
            E::PackFull => StatusCode::CONFLICT,
            E::PackNotFound | E::StickerNotFound => StatusCode::NOT_FOUND,
            E::InvalidOrder => StatusCode::BAD_REQUEST,
            E::InstallLimit => StatusCode::CONFLICT,
            E::Repository(e) => e.to_http_status(),
            E::Storage(e) => e.to_http_status(),
            E::Moderation(e) => e.to_http_status(),
        }
    }
    fn to_user_message(&self) -> String {
        use media::usecases::sticker::StickerError as E;
        match self {
            E::InvalidName => "Pack names need 3 to 64 characters.".to_string(),
            E::InvalidSlug => "Pack links use 3 to 32 lowercase Latin letters, digits or underscores.".to_string(),
            E::InvalidEmoji => "Pick an emoji for the sticker.".to_string(),
            E::Taken => "This pack name or link is already taken.".to_string(),
            E::PackLimit => "You have reached the sticker pack limit.".to_string(),
            E::PackFull => "A pack holds up to 120 stickers.".to_string(),
            E::PackNotFound => "Sticker pack not found".to_string(),
            E::StickerNotFound => "Sticker not found".to_string(),
            E::InvalidOrder => "The new order must list every sticker of the pack once.".to_string(),
            E::InstallLimit => "You can keep up to 200 sticker packs. Remove one first.".to_string(),
            E::Repository(e) => e.to_user_message(),
            E::Storage(e) => e.to_user_message(),
            E::Moderation(e) => e.to_user_message(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        use media::usecases::sticker::StickerError as E;
        match self {
            E::InvalidName => "STICKER_PACK_NAME_INVALID",
            E::InvalidSlug => "STICKER_PACK_SLUG_INVALID",
            E::InvalidEmoji => "STICKER_EMOJI_INVALID",
            E::Taken => "STICKER_PACK_TAKEN",
            E::PackLimit => "STICKER_PACK_LIMIT",
            E::PackFull => "STICKER_PACK_FULL",
            E::PackNotFound => "STICKER_PACK_NOT_FOUND",
            E::StickerNotFound => "STICKER_NOT_FOUND",
            E::InvalidOrder => "STICKER_ORDER_INVALID",
            E::InstallLimit => "STICKER_INSTALL_LIMIT",
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
