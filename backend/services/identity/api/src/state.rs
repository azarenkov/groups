use identity::usecases::{
    auth::{
        confirm_registration::ConfirmRegistration, google_login::GoogleLogin,
        issue_verification_code::IssueVerificationCode, login::LoginUser, logout::Logout,
        refresh_tokens::RefreshTokens, register::RegisterUser,
    },
    user::{
        get_me::GetMe, rename::RenameUser, search_users::SearchUsers,
        update_locale::UpdateUserLocale,
    },
};
use infrastructure_common::{
    hasher::bcrypt_hasher::BcryptHasher,
};
use identity_infrastructure::{
    auth::{google_verifier::GoogleIdTokenVerifier, jwt_token_issuer::JwtTokenIssuer},
    database::postgres::repositories::{
        identity_unit_of_work::PostgresIdentityUnitOfWork,
        refresh_token::PostgresRefreshTokenRepository, user::PostgresUserRepository,
        verification_code::PostgresVerificationCodeRepository,
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
    pub confirm_registration_usecase: ConfirmRegistration<
        PostgresUserRepository,
        BcryptHasher,
        PostgresVerificationCodeRepository,
        PostgresIdentityUnitOfWork,
    >,
    pub refresh_tokens_usecase:
        RefreshTokens<PostgresRefreshTokenRepository, PostgresUserRepository, JwtTokenIssuer>,
    pub logout_usecase: Logout<PostgresRefreshTokenRepository, PostgresIdentityUnitOfWork, JwtTokenIssuer>,
    pub search_users_usecase: SearchUsers<PostgresUserRepository>,
    pub get_me_usecase: GetMe<PostgresUserRepository>,
    pub update_user_locale_usecase: UpdateUserLocale<PostgresUserRepository>,
    pub rename_user_usecase: RenameUser<PostgresUserRepository, PostgresIdentityUnitOfWork, NsfwDetector>,
    pub user_repository: std::sync::Arc<PostgresUserRepository>,
    pub trust_forwarded_headers: bool,
}
