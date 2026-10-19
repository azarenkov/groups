use identity::usecases::user::emoji_status::SetEmojiStatus;
use identity::usecases::user::presence::SetPresencePreference;
use identity::usecases::auth::captcha::CaptchaGate;
use identity_infrastructure::auth::{altcha::AltchaVerifier, redis_captcha_store::RedisCaptchaStore};
use identity_infrastructure::media_client::sticker_lookup::MediaStickerClient;
use identity::usecases::{
    auth::{
        confirm_registration::ConfirmRegistration,
        issue_verification_code::IssueVerificationCode, login::LoginUser, logout::Logout,
        password_reset::{RequestPasswordReset, ResetPassword},
        refresh_tokens::RefreshTokens, register::RegisterUser,
    },
    user::{
        appearance::{GetAppearance, UpdateAppearance},
        hotkey_settings::{GetHotkeySettings, UpdateHotkeySettings},
        notification_settings::{GetNotificationSettings, UpdateNotificationSettings},
        voice_settings::{GetVoiceSettings, UpdateVoiceSettings},
        change_password::ChangePassword, confirm_email_change::ConfirmEmailChange,
        delete_account::DeleteAccount, disable_account::DisableAccount, get_me::GetMe,
        link_preview::GetLinkPreview, rename::RenameUser, request_email_change::RequestEmailChange,
        search_users::SearchUsers, update_locale::UpdateUserLocale, update_profile::UpdateProfile,
    },
};
use infrastructure_common::{
    hasher::bcrypt_hasher::BcryptHasher,
};
use identity_infrastructure::{
    auth::{jwt_token_issuer::JwtTokenIssuer, redis_access_revoker::RedisAccessRevoker},
    database::postgres::repositories::{
        identity_unit_of_work::PostgresIdentityUnitOfWork,
        user::PostgresUserRepository,
    },
    moderation::nsfw_detector::NsfwDetector,
};

pub struct AppState {
    pub captcha_gate: CaptchaGate<AltchaVerifier, RedisCaptchaStore>,
    pub register_user_usecase: RegisterUser<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
        JwtTokenIssuer,
        NsfwDetector,
    >,
    pub issue_verification_code_usecase: IssueVerificationCode<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
    >,
    pub request_password_reset_usecase: RequestPasswordReset<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
    >,
    pub reset_password_usecase: ResetPassword<PostgresIdentityUnitOfWork, BcryptHasher, RedisAccessRevoker>,
    pub login_user_usecase: LoginUser<
        PostgresUserRepository,
        BcryptHasher,
        PostgresIdentityUnitOfWork,
        JwtTokenIssuer,
    >,
    pub confirm_registration_usecase: ConfirmRegistration<BcryptHasher, PostgresIdentityUnitOfWork>,
    pub refresh_tokens_usecase:
        RefreshTokens<PostgresIdentityUnitOfWork, JwtTokenIssuer>,
    pub logout_usecase: Logout<PostgresIdentityUnitOfWork, JwtTokenIssuer>,
    pub search_users_usecase: SearchUsers<PostgresUserRepository>,
    pub get_link_preview_usecase: GetLinkPreview<PostgresUserRepository>,
    pub get_me_usecase: GetMe<PostgresUserRepository>,
    pub update_user_locale_usecase: UpdateUserLocale<PostgresIdentityUnitOfWork>,
    pub update_profile_usecase: UpdateProfile<PostgresIdentityUnitOfWork, NsfwDetector>,
    pub rename_user_usecase: RenameUser<PostgresUserRepository, PostgresIdentityUnitOfWork, NsfwDetector>,
    pub change_password_usecase: ChangePassword<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
        JwtTokenIssuer,
        RedisAccessRevoker,
    >,
    pub request_email_change_usecase: RequestEmailChange<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
    >,
    pub confirm_email_change_usecase: ConfirmEmailChange<PostgresIdentityUnitOfWork, BcryptHasher, JwtTokenIssuer, RedisAccessRevoker>,
    pub disable_account_usecase: DisableAccount<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
        RedisAccessRevoker,
    >,
    pub delete_account_usecase: DeleteAccount<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
        RedisAccessRevoker,
    >,
    pub set_emoji_status_usecase: SetEmojiStatus<PostgresIdentityUnitOfWork, MediaStickerClient>,
    pub get_appearance_usecase: GetAppearance<PostgresUserRepository>,
    pub update_appearance_usecase: UpdateAppearance<PostgresIdentityUnitOfWork>,
    pub set_presence_preference_usecase: SetPresencePreference<PostgresIdentityUnitOfWork>,
    pub get_voice_settings_usecase: GetVoiceSettings<PostgresUserRepository>,
    pub update_voice_settings_usecase: UpdateVoiceSettings<PostgresIdentityUnitOfWork>,
    pub get_notification_settings_usecase: GetNotificationSettings<PostgresUserRepository>,
    pub update_notification_settings_usecase: UpdateNotificationSettings<PostgresIdentityUnitOfWork>,
    pub get_hotkey_settings_usecase: GetHotkeySettings<PostgresUserRepository>,
    pub update_hotkey_settings_usecase: UpdateHotkeySettings<PostgresIdentityUnitOfWork>,
    pub public_app_url: String,
    pub trust_forwarded_headers: bool,
}
