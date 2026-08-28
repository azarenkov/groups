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

use identity::ports::oauth_verifier::OAuthVerifyError;
use identity::ports::token_issuer::TokenIssuerError;
use identity::usecases::auth::confirm_registration::ConfirmRegistrationError;
use identity::usecases::auth::google_login::GoogleLoginError;
use identity::usecases::auth::issue_verification_code::IssueVerificationCodeError;
use identity::usecases::auth::login::LoginError;
use identity::usecases::auth::logout::LogoutError;
use identity::usecases::auth::refresh_tokens::RefreshTokensError;
use identity::usecases::auth::register::RegistrationError;
use identity::usecases::user::change_password::ChangePasswordError;
use identity::usecases::user::confirm_email_change::ConfirmEmailChangeError;
use identity::usecases::user::delete_account::DeleteAccountError;
use identity::usecases::user::disable_account::DisableAccountError;
use identity::usecases::user::get_me::GetMeError;
use identity::usecases::user::request_email_change::RequestEmailChangeError;
use identity::usecases::user::rename::RenameUserError;
use identity::usecases::user::search_users::SearchUsersError;
use identity::usecases::user::update_locale::UpdateLocaleError;
use domain::user::name::UserNameError;
use ports::repository::RepositoryError;

impl HttpMapping for TokenIssuerError {
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

    fn to_error_code(&self) -> &'static str {
        match self {
            TokenIssuerError::Encoding(_) => "TOKEN_ENCODING_FAILED",
            TokenIssuerError::Decoding(_) => "TOKEN_DECODING_FAILED",
            TokenIssuerError::InvalidTokenType => "TOKEN_INVALID_TYPE",
        }
    }
}

impl HttpMapping for LoginError {
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

    fn to_error_code(&self) -> &'static str {
        match self {
            LoginError::Repository(e) => e.to_error_code(),
            LoginError::Hash(e) => e.to_error_code(),
            LoginError::TokenIssuer(e) => e.to_error_code(),
            LoginError::InvalidCredentials => "INVALID_CREDENTIALS",
        }
    }
}

impl HttpMapping for GoogleLoginError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GoogleLoginError::Repository(e) => e.to_http_status(),
            GoogleLoginError::Hash(e) => e.to_http_status(),
            GoogleLoginError::TokenIssuer(e) => e.to_http_status(),
            GoogleLoginError::Verify(OAuthVerifyError::NotConfigured(_)) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            GoogleLoginError::Verify(OAuthVerifyError::Transport(_)) => StatusCode::BAD_GATEWAY,
            GoogleLoginError::Verify(_) => StatusCode::UNAUTHORIZED,
            GoogleLoginError::EmailNotVerified => StatusCode::FORBIDDEN,
            GoogleLoginError::AccountConflict => StatusCode::CONFLICT,
            GoogleLoginError::AccountDeleted => StatusCode::FORBIDDEN,
            GoogleLoginError::UsernameTaken => StatusCode::CONFLICT,
            GoogleLoginError::InvalidName(UserNameError::Reserved) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            GoogleLoginError::InvalidName(_) => StatusCode::BAD_REQUEST,
            GoogleLoginError::NameNotAllowed => StatusCode::UNPROCESSABLE_ENTITY,
            GoogleLoginError::ModerationUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            GoogleLoginError::StateLookupUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            GoogleLoginError::Repository(e) => e.to_user_message(),
            GoogleLoginError::Hash(e) => e.to_user_message(),
            GoogleLoginError::TokenIssuer(e) => e.to_user_message(),
            GoogleLoginError::Verify(OAuthVerifyError::NotConfigured(_)) => {
                "Google login is not configured".to_string()
            }
            GoogleLoginError::Verify(OAuthVerifyError::Transport(_)) => {
                "Could not reach Google to verify the token".to_string()
            }
            GoogleLoginError::Verify(_) => "Invalid Google token".to_string(),
            GoogleLoginError::EmailNotVerified => {
                "Google account email is not verified".to_string()
            }
            GoogleLoginError::AccountConflict => {
                "This email is already linked to a different Google account".to_string()
            }
            GoogleLoginError::AccountDeleted => "This account has been deleted".to_string(),
            GoogleLoginError::UsernameTaken => "This username is already taken".to_string(),
            GoogleLoginError::InvalidName(UserNameError::Reserved)
            | GoogleLoginError::NameNotAllowed => "This username is not allowed.".to_string(),
            GoogleLoginError::InvalidName(e) => e.to_string(),
            GoogleLoginError::ModerationUnavailable(_) => {
                "Username moderation is temporarily unavailable. Try again later.".to_string()
            }
            GoogleLoginError::StateLookupUnavailable(_) => {
                "Account linking is temporarily unavailable. Try again later.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            GoogleLoginError::Repository(e) => e.to_error_code(),
            GoogleLoginError::Hash(e) => e.to_error_code(),
            GoogleLoginError::TokenIssuer(e) => e.to_error_code(),
            GoogleLoginError::Verify(OAuthVerifyError::NotConfigured(_)) => {
                "GOOGLE_NOT_CONFIGURED"
            }
            GoogleLoginError::Verify(OAuthVerifyError::Transport(_)) => {
                "GOOGLE_VERIFY_UNAVAILABLE"
            }
            GoogleLoginError::Verify(_) => "GOOGLE_TOKEN_INVALID",
            GoogleLoginError::EmailNotVerified => "GOOGLE_EMAIL_NOT_VERIFIED",
            GoogleLoginError::AccountConflict => "GOOGLE_ACCOUNT_CONFLICT",
            GoogleLoginError::AccountDeleted => "ACCOUNT_DELETED",
            GoogleLoginError::UsernameTaken => "GOOGLE_USERNAME_TAKEN",
            GoogleLoginError::InvalidName(UserNameError::Reserved)
            | GoogleLoginError::NameNotAllowed => "NAME_NOT_ALLOWED",
            GoogleLoginError::InvalidName(_) => "NAME_INVALID_FORMAT",
            GoogleLoginError::ModerationUnavailable(_) => "MODERATION_UNAVAILABLE",
            GoogleLoginError::StateLookupUnavailable(_) => "STATE_LOOKUP_UNAVAILABLE",
        }
    }
}

impl HttpMapping for RefreshTokensError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RefreshTokensError::Repository(e) => e.to_http_status(),
            RefreshTokensError::TokenIssuer(e) => e.to_http_status(),
            RefreshTokensError::RefreshTokenNotFound | RefreshTokensError::UserNotFound => {
                StatusCode::NOT_FOUND
            }
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
            RefreshTokensError::UserNotFound => "User not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            RefreshTokensError::Repository(e) => e.to_error_code(),
            RefreshTokensError::TokenIssuer(e) => e.to_error_code(),
            RefreshTokensError::RefreshTokenNotFound => "REFRESH_TOKEN_NOT_FOUND",
            RefreshTokensError::RefreshTokenRevoked => "REFRESH_TOKEN_REVOKED",
            RefreshTokensError::RefreshTokenExpired => "REFRESH_TOKEN_EXPIRED",
            RefreshTokensError::UserMismatch => "REFRESH_TOKEN_USER_MISMATCH",
            RefreshTokensError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl HttpMapping for LogoutError {
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

    fn to_error_code(&self) -> &'static str {
        match self {
            LogoutError::Repository(e) => e.to_error_code(),
            LogoutError::TokenIssuer(e) => e.to_error_code(),
            LogoutError::RefreshTokenNotFound => "REFRESH_TOKEN_NOT_FOUND",
            LogoutError::UserMismatch => "REFRESH_TOKEN_USER_MISMATCH",
        }
    }
}

impl HttpMapping for ConfirmRegistrationError {
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

    fn to_error_code(&self) -> &'static str {
        match self {
            ConfirmRegistrationError::UserAlreadyVerified => "USER_ALREADY_VERIFIED",
            ConfirmRegistrationError::TooManyAttempts => "VERIFICATION_TOO_MANY_ATTEMPTS",
            ConfirmRegistrationError::VerificationCodeExpired => "VERIFICATION_CODE_EXPIRED",
            ConfirmRegistrationError::BadVerificationCode => "VERIFICATION_CODE_INVALID",
            ConfirmRegistrationError::NotRegistration => "VERIFICATION_CODE_NOT_REGISTRATION",
            ConfirmRegistrationError::UserNotFound => "USER_NOT_FOUND",
            ConfirmRegistrationError::Repository(e) => e.to_error_code(),
            ConfirmRegistrationError::Hash(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for RegistrationError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RegistrationError::Repository(e) => e.to_http_status(),
            RegistrationError::Hash(e) => e.to_http_status(),
            RegistrationError::TokenIssuer(e) => e.to_http_status(),
            RegistrationError::InvalidName(UserNameError::Reserved) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            RegistrationError::InvalidName(_) => StatusCode::BAD_REQUEST,
            RegistrationError::NameTaken => StatusCode::CONFLICT,
            RegistrationError::NameNotAllowed => StatusCode::UNPROCESSABLE_ENTITY,
            RegistrationError::ModerationUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RegistrationError::Repository(e) => e.to_user_message(),
            RegistrationError::Hash(e) => e.to_user_message(),
            RegistrationError::TokenIssuer(e) => e.to_user_message(),
            RegistrationError::InvalidName(UserNameError::Reserved)
            | RegistrationError::NameNotAllowed => "This username is not allowed.".to_string(),
            RegistrationError::InvalidName(e) => e.to_string(),
            RegistrationError::NameTaken => "This username is already taken".to_string(),
            RegistrationError::ModerationUnavailable(_) => {
                "Username moderation is temporarily unavailable. Try again later.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            RegistrationError::Repository(e) => match e {
                RepositoryError::AlreadyExists => "EMAIL_ALREADY_TAKEN",
                _ => e.to_error_code(),
            },
            RegistrationError::Hash(e) => e.to_error_code(),
            RegistrationError::TokenIssuer(e) => e.to_error_code(),
            RegistrationError::InvalidName(UserNameError::Reserved)
            | RegistrationError::NameNotAllowed => "NAME_NOT_ALLOWED",
            RegistrationError::InvalidName(_) => "NAME_INVALID_FORMAT",
            RegistrationError::NameTaken => "NAME_TAKEN",
            RegistrationError::ModerationUnavailable(_) => "MODERATION_UNAVAILABLE",
        }
    }
}

impl HttpMapping for IssueVerificationCodeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            IssueVerificationCodeError::Repository(e) => e.to_http_status(),
            IssueVerificationCodeError::Hash(e) => e.to_http_status(),
            IssueVerificationCodeError::UnsupportedChannel => StatusCode::BAD_REQUEST,
            IssueVerificationCodeError::UserAlreadyVerified => StatusCode::CONFLICT,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            IssueVerificationCodeError::Repository(e) => e.to_user_message(),
            IssueVerificationCodeError::Hash(e) => e.to_user_message(),
            IssueVerificationCodeError::UnsupportedChannel => {
                "Unsupported verification channel".to_string()
            }
            IssueVerificationCodeError::UserAlreadyVerified => "User already verified".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            IssueVerificationCodeError::Repository(e) => e.to_error_code(),
            IssueVerificationCodeError::Hash(e) => e.to_error_code(),
            IssueVerificationCodeError::UnsupportedChannel => "VERIFICATION_CHANNEL_UNSUPPORTED",
            IssueVerificationCodeError::UserAlreadyVerified => "USER_ALREADY_VERIFIED",
        }
    }
}

impl HttpMapping for GetMeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GetMeError::Repository(e) => e.to_http_status(),
            GetMeError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            GetMeError::Repository(e) => e.to_user_message(),
            GetMeError::NotFound => "User not found".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            GetMeError::Repository(e) => e.to_error_code(),
            GetMeError::NotFound => "USER_NOT_FOUND",
        }
    }
}

impl HttpMapping for UpdateLocaleError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            UpdateLocaleError::Repository(e) => e.to_http_status(),
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            UpdateLocaleError::Repository(e) => e.to_user_message(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            UpdateLocaleError::Repository(e) => e.to_error_code(),
        }
    }
}

impl HttpMapping for SearchUsersError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            SearchUsersError::Repository(e) => e.to_http_status(),
            SearchUsersError::QueryTooShort => StatusCode::BAD_REQUEST,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            SearchUsersError::Repository(e) => e.to_user_message(),
            SearchUsersError::QueryTooShort => "Search query too short".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            SearchUsersError::Repository(e) => e.to_error_code(),
            SearchUsersError::QueryTooShort => "QUERY_TOO_SHORT",
        }
    }
}

impl HttpMapping for RenameUserError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RenameUserError::Repository(e) => e.to_http_status(),
            RenameUserError::InvalidName(UserNameError::Reserved) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            RenameUserError::InvalidName(_) => StatusCode::BAD_REQUEST,
            RenameUserError::NameTaken => StatusCode::CONFLICT,
            RenameUserError::NameNotAllowed => StatusCode::UNPROCESSABLE_ENTITY,
            RenameUserError::ModerationUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            RenameUserError::Repository(e) => e.to_user_message(),
            RenameUserError::InvalidName(UserNameError::Reserved)
            | RenameUserError::NameNotAllowed => "This username is not allowed.".to_string(),
            RenameUserError::InvalidName(e) => e.to_string(),
            RenameUserError::NameTaken => "This username is already taken".to_string(),
            RenameUserError::ModerationUnavailable(_) => {
                "Username moderation is temporarily unavailable. Try again later.".to_string()
            }
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            RenameUserError::Repository(e) => e.to_error_code(),
            RenameUserError::InvalidName(UserNameError::Reserved)
            | RenameUserError::NameNotAllowed => "NAME_NOT_ALLOWED",
            RenameUserError::InvalidName(_) => "NAME_INVALID_FORMAT",
            RenameUserError::NameTaken => "NAME_TAKEN",
            RenameUserError::ModerationUnavailable(_) => "MODERATION_UNAVAILABLE",
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

impl HttpMapping for ChangePasswordError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ChangePasswordError::Repository(e) => e.to_http_status(),
            ChangePasswordError::Hash(e) => e.to_http_status(),
            ChangePasswordError::TokenIssuer(e) => e.to_http_status(),
            ChangePasswordError::InvalidPassword => StatusCode::FORBIDDEN,
            ChangePasswordError::SamePassword => StatusCode::BAD_REQUEST,
            ChangePasswordError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ChangePasswordError::Repository(e) => e.to_user_message(),
            ChangePasswordError::Hash(e) => e.to_user_message(),
            ChangePasswordError::TokenIssuer(e) => e.to_user_message(),
            ChangePasswordError::InvalidPassword => "Current password is incorrect".to_string(),
            ChangePasswordError::SamePassword => "New password must differ from the current one".to_string(),
            ChangePasswordError::UserNotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ChangePasswordError::Repository(e) => e.to_error_code(),
            ChangePasswordError::Hash(e) => e.to_error_code(),
            ChangePasswordError::TokenIssuer(e) => e.to_error_code(),
            ChangePasswordError::InvalidPassword => "INVALID_PASSWORD",
            ChangePasswordError::SamePassword => "SAME_PASSWORD",
            ChangePasswordError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl HttpMapping for RequestEmailChangeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RequestEmailChangeError::Repository(e) => e.to_http_status(),
            RequestEmailChangeError::Hash(e) => e.to_http_status(),
            RequestEmailChangeError::InvalidPassword => StatusCode::FORBIDDEN,
            RequestEmailChangeError::EmailTaken => StatusCode::CONFLICT,
            RequestEmailChangeError::SameEmail => StatusCode::BAD_REQUEST,
            RequestEmailChangeError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            RequestEmailChangeError::Repository(e) => e.to_user_message(),
            RequestEmailChangeError::Hash(e) => e.to_user_message(),
            RequestEmailChangeError::InvalidPassword => "Password is incorrect".to_string(),
            RequestEmailChangeError::EmailTaken => "This email is already in use".to_string(),
            RequestEmailChangeError::SameEmail => "This is already your email".to_string(),
            RequestEmailChangeError::UserNotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            RequestEmailChangeError::Repository(e) => e.to_error_code(),
            RequestEmailChangeError::Hash(e) => e.to_error_code(),
            RequestEmailChangeError::InvalidPassword => "INVALID_PASSWORD",
            RequestEmailChangeError::EmailTaken => "EMAIL_TAKEN",
            RequestEmailChangeError::SameEmail => "SAME_EMAIL",
            RequestEmailChangeError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl HttpMapping for ConfirmEmailChangeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            ConfirmEmailChangeError::Repository(e) => e.to_http_status(),
            ConfirmEmailChangeError::Hash(e) => e.to_http_status(),
            ConfirmEmailChangeError::NoPendingChange => StatusCode::CONFLICT,
            ConfirmEmailChangeError::TooManyAttempts => StatusCode::TOO_MANY_REQUESTS,
            ConfirmEmailChangeError::CodeExpired => StatusCode::GONE,
            ConfirmEmailChangeError::BadCode => StatusCode::BAD_REQUEST,
            ConfirmEmailChangeError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            ConfirmEmailChangeError::Repository(e) => e.to_user_message(),
            ConfirmEmailChangeError::Hash(e) => e.to_user_message(),
            ConfirmEmailChangeError::NoPendingChange => "No pending email change".to_string(),
            ConfirmEmailChangeError::TooManyAttempts => "Too many attempts, request a new code".to_string(),
            ConfirmEmailChangeError::CodeExpired => "Verification code has expired".to_string(),
            ConfirmEmailChangeError::BadCode => "Incorrect verification code".to_string(),
            ConfirmEmailChangeError::UserNotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            ConfirmEmailChangeError::Repository(e) => e.to_error_code(),
            ConfirmEmailChangeError::Hash(e) => e.to_error_code(),
            ConfirmEmailChangeError::NoPendingChange => "NO_PENDING_EMAIL_CHANGE",
            ConfirmEmailChangeError::TooManyAttempts => "TOO_MANY_ATTEMPTS",
            ConfirmEmailChangeError::CodeExpired => "CODE_EXPIRED",
            ConfirmEmailChangeError::BadCode => "BAD_CODE",
            ConfirmEmailChangeError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl HttpMapping for DisableAccountError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DisableAccountError::Repository(e) => e.to_http_status(),
            DisableAccountError::Hash(e) => e.to_http_status(),
            DisableAccountError::AccessRevoker(_) => StatusCode::SERVICE_UNAVAILABLE,
            DisableAccountError::InvalidPassword => StatusCode::FORBIDDEN,
            DisableAccountError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DisableAccountError::Repository(e) => e.to_user_message(),
            DisableAccountError::Hash(e) => e.to_user_message(),
            DisableAccountError::AccessRevoker(_) => "Session revocation is temporarily unavailable".to_string(),
            DisableAccountError::InvalidPassword => "Password is incorrect".to_string(),
            DisableAccountError::UserNotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DisableAccountError::Repository(e) => e.to_error_code(),
            DisableAccountError::Hash(e) => e.to_error_code(),
            DisableAccountError::AccessRevoker(_) => "REVOCATION_UNAVAILABLE",
            DisableAccountError::InvalidPassword => "INVALID_PASSWORD",
            DisableAccountError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}

impl HttpMapping for DeleteAccountError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            DeleteAccountError::Repository(e) => e.to_http_status(),
            DeleteAccountError::Hash(e) => e.to_http_status(),
            DeleteAccountError::AccessRevoker(_) => StatusCode::SERVICE_UNAVAILABLE,
            DeleteAccountError::InvalidPassword => StatusCode::FORBIDDEN,
            DeleteAccountError::UserNotFound => StatusCode::NOT_FOUND,
        }
    }
    fn to_user_message(&self) -> String {
        match self {
            DeleteAccountError::Repository(e) => e.to_user_message(),
            DeleteAccountError::Hash(e) => e.to_user_message(),
            DeleteAccountError::AccessRevoker(_) => "Session revocation is temporarily unavailable".to_string(),
            DeleteAccountError::InvalidPassword => "Password is incorrect".to_string(),
            DeleteAccountError::UserNotFound => "User not found".to_string(),
        }
    }
    fn to_error_code(&self) -> &'static str {
        match self {
            DeleteAccountError::Repository(e) => e.to_error_code(),
            DeleteAccountError::Hash(e) => e.to_error_code(),
            DeleteAccountError::AccessRevoker(_) => "REVOCATION_UNAVAILABLE",
            DeleteAccountError::InvalidPassword => "INVALID_PASSWORD",
            DeleteAccountError::UserNotFound => "USER_NOT_FOUND",
        }
    }
}
