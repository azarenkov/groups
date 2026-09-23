use identity::usecases::entitlements::{
    get_entitlements::GetEntitlements, grant_entitlements::GrantEntitlements,
    set_badge_style::SetBadgeStyle,
};
use identity::usecases::user::activity::{ClearActivity, GetActivities, SetActivity};
use identity_infrastructure::database::postgres::repositories::entitlements::PostgresEntitlementsRepository;
use std::sync::Arc;
use identity_infrastructure::activity::redis_activity_store::RedisActivityStore;
use identity_infrastructure::media_client::activity_audience::MessagingAudienceClient;
use identity_infrastructure::media_client::block_lookup::MessagingBlockClient;
use identity::usecases::{
    connections::{
        complete_connection::CompleteConnection, list_connections::ListConnections,
        list_user_connections::ListUserConnections,
        remove_connection::RemoveConnection, start_connection::StartConnection,
    },
    auth::{
        confirm_registration::ConfirmRegistration, google_login::GoogleLogin,
        issue_verification_code::IssueVerificationCode, login::LoginUser, logout::Logout,
        refresh_tokens::RefreshTokens, register::RegisterUser,
    },
    user::{
        appearance::{GetAppearance, UpdateAppearance},
        hotkey_settings::{GetHotkeySettings, UpdateHotkeySettings},
        notification_settings::{GetNotificationSettings, UpdateNotificationSettings},
        voice_settings::{GetVoiceSettings, UpdateVoiceSettings},
        change_password::ChangePassword, confirm_email_change::ConfirmEmailChange,
        delete_account::DeleteAccount, disable_account::DisableAccount, get_me::GetMe,
        rename::RenameUser, request_email_change::RequestEmailChange,
        search_users::SearchUsers, update_locale::UpdateUserLocale, update_profile::UpdateProfile,
    },
};
use infrastructure_common::{
    hasher::bcrypt_hasher::BcryptHasher,
};
use identity_infrastructure::{
    auth::{github_connector::GithubOAuthConnector, google_verifier::GoogleIdTokenVerifier, jwt_token_issuer::JwtTokenIssuer, redis_access_revoker::RedisAccessRevoker, steam_connector::SteamOpenIdConnector},
    database::postgres::repositories::{
        identity_unit_of_work::PostgresIdentityUnitOfWork,
        user::PostgresUserRepository,
    },
    media_client::user_state::MessagingStateClient,
    moderation::nsfw_detector::NsfwDetector,
};

pub struct AppState {
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
    pub login_user_usecase: LoginUser<
        PostgresUserRepository,
        BcryptHasher,
        PostgresIdentityUnitOfWork,
        JwtTokenIssuer,
    >,
    pub google_login_usecase: GoogleLogin<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        JwtTokenIssuer,
        GoogleIdTokenVerifier,
        BcryptHasher,
        NsfwDetector,
        MessagingStateClient,
    >,
    pub confirm_registration_usecase: ConfirmRegistration<BcryptHasher, PostgresIdentityUnitOfWork>,
    pub refresh_tokens_usecase:
        RefreshTokens<PostgresIdentityUnitOfWork, JwtTokenIssuer>,
    pub logout_usecase: Logout<PostgresIdentityUnitOfWork, JwtTokenIssuer>,
    pub search_users_usecase: SearchUsers<PostgresUserRepository>,
    pub get_me_usecase: GetMe<PostgresUserRepository>,
    pub update_user_locale_usecase: UpdateUserLocale<PostgresIdentityUnitOfWork>,
    pub activity_store: Arc<RedisActivityStore>,
    pub update_profile_usecase: UpdateProfile<PostgresIdentityUnitOfWork, RedisActivityStore, NsfwDetector>,
    pub rename_user_usecase: RenameUser<PostgresUserRepository, PostgresIdentityUnitOfWork, NsfwDetector>,
    pub change_password_usecase: ChangePassword<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
        JwtTokenIssuer,
        GoogleIdTokenVerifier,
        RedisAccessRevoker,
    >,
    pub request_email_change_usecase: RequestEmailChange<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
        GoogleIdTokenVerifier,
    >,
    pub confirm_email_change_usecase: ConfirmEmailChange<PostgresIdentityUnitOfWork, BcryptHasher>,
    pub disable_account_usecase: DisableAccount<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
        RedisAccessRevoker,
        GoogleIdTokenVerifier,
        RedisActivityStore,
    >,
    pub delete_account_usecase: DeleteAccount<
        PostgresUserRepository,
        PostgresIdentityUnitOfWork,
        BcryptHasher,
        RedisAccessRevoker,
        GoogleIdTokenVerifier,
        RedisActivityStore,
    >,
    pub get_entitlements_usecase: GetEntitlements<PostgresEntitlementsRepository>,
    pub grant_entitlements_usecase: GrantEntitlements<PostgresIdentityUnitOfWork>,
    pub set_badge_style_usecase: SetBadgeStyle<PostgresIdentityUnitOfWork>,
    pub get_appearance_usecase: GetAppearance<PostgresUserRepository>,
    pub update_appearance_usecase: UpdateAppearance<PostgresIdentityUnitOfWork>,
    pub set_activity_usecase: SetActivity<RedisActivityStore, PostgresIdentityUnitOfWork>,
    pub clear_activity_usecase: ClearActivity<RedisActivityStore, PostgresIdentityUnitOfWork>,
    pub get_activities_usecase: GetActivities<RedisActivityStore, MessagingAudienceClient>,
    pub get_voice_settings_usecase: GetVoiceSettings<PostgresUserRepository, PostgresEntitlementsRepository>,
    pub update_voice_settings_usecase: UpdateVoiceSettings<PostgresIdentityUnitOfWork>,
    pub get_notification_settings_usecase: GetNotificationSettings<PostgresUserRepository>,
    pub update_notification_settings_usecase: UpdateNotificationSettings<PostgresIdentityUnitOfWork>,
    pub get_hotkey_settings_usecase: GetHotkeySettings<PostgresUserRepository>,
    pub update_hotkey_settings_usecase: UpdateHotkeySettings<PostgresIdentityUnitOfWork>,
    pub start_connection_usecase:
        StartConnection<PostgresUserRepository, GithubOAuthConnector, SteamOpenIdConnector>,
    pub complete_connection_usecase:
        CompleteConnection<PostgresUserRepository, GithubOAuthConnector, SteamOpenIdConnector>,
    pub list_connections_usecase: ListConnections<PostgresUserRepository>,
    pub list_user_connections_usecase:
        ListUserConnections<PostgresUserRepository, MessagingBlockClient>,
    pub remove_connection_usecase: RemoveConnection<PostgresUserRepository>,
    pub public_app_url: String,
    pub user_repository: std::sync::Arc<PostgresUserRepository>,
    pub trust_forwarded_headers: bool,
}
