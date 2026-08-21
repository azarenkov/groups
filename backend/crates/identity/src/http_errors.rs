use shared::api_error::{StatusCode, ToHttpStatus};
use crate::ports::oauth_verifier::OAuthVerifyError;
use crate::ports::token_issuer::TokenIssuerError;
use crate::usecases::auth::confirm_registration::ConfirmRegistrationError;
use crate::usecases::auth::google_login::GoogleLoginError;
use crate::usecases::auth::issue_verification_code::IssueVerificationCodeError;
use crate::usecases::auth::login::LoginError;
use crate::usecases::auth::logout::LogoutError;
use crate::usecases::auth::refresh_tokens::RefreshTokensError;
use crate::usecases::auth::register::RegistrationError;
use crate::usecases::user::get_me::GetMeError;
use crate::usecases::user::rename::RenameUserError;
use crate::usecases::user::search_users::SearchUsersError;
use crate::usecases::user::update_locale::UpdateLocaleError;
use domain::shared::repository::error::RepositoryError;
use domain::user::name::UserNameError;

impl ToHttpStatus for TokenIssuerError {
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

impl ToHttpStatus for LoginError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LoginError::Repository(e) => e.to_http_status(),
            LoginError::Hash(e) => e.to_http_status(),
            LoginError::Outbox(e) => e.to_http_status(),
            LoginError::TokenIssuer(e) => e.to_http_status(),
            LoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LoginError::Repository(e) => e.to_user_message(),
            LoginError::Hash(e) => e.to_user_message(),
            LoginError::Outbox(e) => e.to_user_message(),
            LoginError::TokenIssuer(e) => e.to_user_message(),
            LoginError::InvalidCredentials => "Invalid credentials".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            LoginError::Repository(e) => e.to_error_code(),
            LoginError::Hash(e) => e.to_error_code(),
            LoginError::Outbox(e) => e.to_error_code(),
            LoginError::TokenIssuer(e) => e.to_error_code(),
            LoginError::InvalidCredentials => "INVALID_CREDENTIALS",
        }
    }
}

impl ToHttpStatus for GoogleLoginError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            GoogleLoginError::Repository(e) => e.to_http_status(),
            GoogleLoginError::Hash(e) => e.to_http_status(),
            GoogleLoginError::Outbox(e) => e.to_http_status(),
            GoogleLoginError::TokenIssuer(e) => e.to_http_status(),
            GoogleLoginError::Verify(OAuthVerifyError::NotConfigured(_)) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            GoogleLoginError::Verify(OAuthVerifyError::Transport(_)) => StatusCode::BAD_GATEWAY,
            GoogleLoginError::Verify(_) => StatusCode::UNAUTHORIZED,
            GoogleLoginError::EmailNotVerified => StatusCode::FORBIDDEN,
            GoogleLoginError::AccountConflict => StatusCode::CONFLICT,
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
            GoogleLoginError::Outbox(e) => e.to_user_message(),
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
            GoogleLoginError::Outbox(e) => e.to_error_code(),
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
            GoogleLoginError::UsernameTaken => "GOOGLE_USERNAME_TAKEN",
            GoogleLoginError::InvalidName(UserNameError::Reserved)
            | GoogleLoginError::NameNotAllowed => "NAME_NOT_ALLOWED",
            GoogleLoginError::InvalidName(_) => "NAME_INVALID_FORMAT",
            GoogleLoginError::ModerationUnavailable(_) => "MODERATION_UNAVAILABLE",
            GoogleLoginError::StateLookupUnavailable(_) => "STATE_LOOKUP_UNAVAILABLE",
        }
    }
}

impl ToHttpStatus for RefreshTokensError {
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

impl ToHttpStatus for LogoutError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            LogoutError::Repository(e) => e.to_http_status(),
            LogoutError::Outbox(e) => e.to_http_status(),
            LogoutError::TokenIssuer(e) => e.to_http_status(),
            LogoutError::RefreshTokenNotFound => StatusCode::NOT_FOUND,
            LogoutError::UserMismatch => StatusCode::UNAUTHORIZED,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            LogoutError::Repository(e) => e.to_user_message(),
            LogoutError::Outbox(e) => e.to_user_message(),
            LogoutError::TokenIssuer(e) => e.to_user_message(),
            LogoutError::RefreshTokenNotFound => "Refresh token not found".to_string(),
            LogoutError::UserMismatch => "Refresh token user mismatch".to_string(),
        }
    }

    fn to_error_code(&self) -> &'static str {
        match self {
            LogoutError::Repository(e) => e.to_error_code(),
            LogoutError::Outbox(e) => e.to_error_code(),
            LogoutError::TokenIssuer(e) => e.to_error_code(),
            LogoutError::RefreshTokenNotFound => "REFRESH_TOKEN_NOT_FOUND",
            LogoutError::UserMismatch => "REFRESH_TOKEN_USER_MISMATCH",
        }
    }
}

impl ToHttpStatus for ConfirmRegistrationError {
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
            ConfirmRegistrationError::Outbox(e) => e.to_http_status(),
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
            ConfirmRegistrationError::Outbox(e) => e.to_user_message(),
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
            ConfirmRegistrationError::Outbox(e) => e.to_error_code(),
        }
    }
}

impl ToHttpStatus for RegistrationError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RegistrationError::Repository(e) => e.to_http_status(),
            RegistrationError::Hash(e) => e.to_http_status(),
            RegistrationError::Outbox(e) => e.to_http_status(),
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
            RegistrationError::Outbox(e) => e.to_user_message(),
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
            RegistrationError::Outbox(e) => e.to_error_code(),
            RegistrationError::TokenIssuer(e) => e.to_error_code(),
            RegistrationError::InvalidName(UserNameError::Reserved)
            | RegistrationError::NameNotAllowed => "NAME_NOT_ALLOWED",
            RegistrationError::InvalidName(_) => "NAME_INVALID_FORMAT",
            RegistrationError::NameTaken => "NAME_TAKEN",
            RegistrationError::ModerationUnavailable(_) => "MODERATION_UNAVAILABLE",
        }
    }
}

impl ToHttpStatus for IssueVerificationCodeError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            IssueVerificationCodeError::Repository(e) => e.to_http_status(),
            IssueVerificationCodeError::Hash(e) => e.to_http_status(),
            IssueVerificationCodeError::Outbox(e) => e.to_http_status(),
            IssueVerificationCodeError::UnsupportedChannel => StatusCode::BAD_REQUEST,
            IssueVerificationCodeError::UserAlreadyVerified => StatusCode::CONFLICT,
        }
    }

    fn to_user_message(&self) -> String {
        match self {
            IssueVerificationCodeError::Repository(e) => e.to_user_message(),
            IssueVerificationCodeError::Hash(e) => e.to_user_message(),
            IssueVerificationCodeError::Outbox(e) => e.to_user_message(),
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
            IssueVerificationCodeError::Outbox(e) => e.to_error_code(),
            IssueVerificationCodeError::UnsupportedChannel => "VERIFICATION_CHANNEL_UNSUPPORTED",
            IssueVerificationCodeError::UserAlreadyVerified => "USER_ALREADY_VERIFIED",
        }
    }
}

impl ToHttpStatus for GetMeError {
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

impl ToHttpStatus for UpdateLocaleError {
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

impl ToHttpStatus for SearchUsersError {
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

impl ToHttpStatus for RenameUserError {
    fn to_http_status(&self) -> StatusCode {
        match self {
            RenameUserError::Repository(e) => e.to_http_status(),
            RenameUserError::Outbox(e) => e.to_http_status(),
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
            RenameUserError::Outbox(e) => e.to_user_message(),
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
            RenameUserError::Outbox(e) => e.to_error_code(),
            RenameUserError::InvalidName(UserNameError::Reserved)
            | RenameUserError::NameNotAllowed => "NAME_NOT_ALLOWED",
            RenameUserError::InvalidName(_) => "NAME_INVALID_FORMAT",
            RenameUserError::NameTaken => "NAME_TAKEN",
            RenameUserError::ModerationUnavailable(_) => "MODERATION_UNAVAILABLE",
        }
    }
}

shared::impl_from_for_api_error!(
    ConfirmRegistrationError,
    GetMeError,
    GoogleLoginError,
    IssueVerificationCodeError,
    LoginError,
    LogoutError,
    RefreshTokensError,
    RegistrationError,
    RenameUserError,
    SearchUsersError,
    UpdateLocaleError,
);
