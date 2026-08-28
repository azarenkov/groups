use std::{error::Error, sync::Arc};

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use identity::usecases::{
    auth::{
        confirm_registration::ConfirmRegistration, google_login::GoogleLogin,
        issue_verification_code::IssueVerificationCode, login::LoginUser, logout::Logout,
        refresh_tokens::RefreshTokens, register::RegisterUser,
    },
    user::{
        get_me::GetMe, rename::RenameUser, search_users::SearchUsers,
        update_locale::UpdateUserLocale,
        change_password::ChangePassword, confirm_email_change::ConfirmEmailChange,
        delete_account::DeleteAccount, disable_account::DisableAccount,
        reauth::Reauthenticator, request_email_change::RequestEmailChange,
    },
};
use domain::user::name::ReservedNames;
use infrastructure_common::{
    config::{
        google::GoogleOAuthConfig, nsfw::NsfwConfig, postgres::PostgresConfig,
        username::UsernameConfig,
    },
    hasher::bcrypt_hasher::BcryptHasher,
};
use identity_infrastructure::{
    auth::{google_verifier::GoogleIdTokenVerifier, jwt_token_issuer::JwtTokenIssuer, redis_access_revoker::RedisAccessRevoker},
    database::postgres::{
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            identity_unit_of_work::PostgresIdentityUnitOfWork,
            user::PostgresUserRepository,
        },
    },
    media_client::user_state::MessagingStateClient,
    moderation::nsfw_detector::NsfwDetector,
};
use api_common::{
    health::Readiness,
    middleware::request_id::RequestIdMiddleware,
    rate_limiting::config::RateLimitingConfig,
};

use api_common::config::ServerConfig;

use crate::{routes, state::AppState};

pub async fn run() -> Result<(), Box<dyn Error>> {
    infrastructure_common::observability::init("identity-service");

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let google_config = GoogleOAuthConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;
    let nsfw_config = NsfwConfig::from_env()?;
    let username_config = UsernameConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config).await?;

    let user_repository = Arc::new(PostgresUserRepository::new(postgres_pool.clone()));
    let unit_of_work = Arc::new(PostgresIdentityUnitOfWork::new(postgres_pool.clone()));

    let hasher = Arc::new(BcryptHasher::default());
    let token_issuer = Arc::new(JwtTokenIssuer::new(server_config.jwt_secret.clone()));
    let nsfw_detector = Arc::new(NsfwDetector::new(&nsfw_config));
    let reserved_names = Arc::new(ReservedNames::new(username_config.extra_reserved()));

    let google_audiences = google_config.audiences();
    if google_audiences.is_empty() {
        tracing::warn!(
            "GOOGLE_CLIENT_ID is not set; /v1/auth/google will reject requests with GOOGLE_NOT_CONFIGURED"
        );
    }
    let google_token_verifier = Arc::new(GoogleIdTokenVerifier::new(google_audiences));
    let messaging_internal_url = std::env::var("MESSAGING_INTERNAL_URL")
        .unwrap_or_else(|_| "http://messaging-service:8080".to_string());
    let user_state_lookup = Arc::new(MessagingStateClient::new(&messaging_internal_url));

    let access_revocations = api_common::middleware::revocation::AccessRevocations::from_env().await;
    let access_revocations_data = web::Data::new(access_revocations.clone());
    let access_revoker = Arc::new(RedisAccessRevoker::new(access_revocations));
    let reauthenticator = Arc::new(Reauthenticator::new(
        user_repository.clone(),
        hasher.clone(),
        google_token_verifier.clone(),
    ));

    let app_state = web::Data::new(AppState {
        register_user_usecase: RegisterUser::new(
            user_repository.clone(),
            unit_of_work.clone(),
            hasher.clone(),
            token_issuer.clone(),
            nsfw_detector.clone(),
            reserved_names.clone(),
        ),
        issue_verification_code_usecase: IssueVerificationCode::new(
            user_repository.clone(),
            unit_of_work.clone(),
            hasher.clone(),
        ),
        login_user_usecase: LoginUser::new(
            user_repository.clone(),
            hasher.clone(),
            unit_of_work.clone(),
            token_issuer.clone(),
        ),
        google_login_usecase: GoogleLogin::new(
            user_repository.clone(),
            unit_of_work.clone(),
            token_issuer.clone(),
            google_token_verifier.clone(),
            hasher.clone(),
            nsfw_detector.clone(),
            reserved_names.clone(),
            user_state_lookup.clone(),
        ),
        confirm_registration_usecase: ConfirmRegistration::new(hasher.clone(), unit_of_work.clone()),
        refresh_tokens_usecase: RefreshTokens::new(
            unit_of_work.clone(),
            user_repository.clone(),
            token_issuer.clone(),
        ),
        logout_usecase: Logout::new(unit_of_work.clone(), token_issuer.clone()),
        search_users_usecase: SearchUsers::new(user_repository.clone()),
        get_me_usecase: GetMe::new(user_repository.clone()),
        update_user_locale_usecase: UpdateUserLocale::new(unit_of_work.clone()),
        rename_user_usecase: RenameUser::new(
            user_repository.clone(),
            unit_of_work.clone(),
            nsfw_detector.clone(),
            reserved_names.clone(),
        ),
        change_password_usecase: ChangePassword::new(
            user_repository.clone(),
            unit_of_work.clone(),
            hasher.clone(),
            token_issuer.clone(),
            reauthenticator.clone(),
            access_revoker.clone(),
        ),
        request_email_change_usecase: RequestEmailChange::new(
            user_repository.clone(),
            unit_of_work.clone(),
            hasher.clone(),
            reauthenticator.clone(),
        ),
        confirm_email_change_usecase: ConfirmEmailChange::new(unit_of_work.clone(), hasher.clone()),
        disable_account_usecase: DisableAccount::new(
            user_repository.clone(),
            unit_of_work.clone(),
            access_revoker.clone(),
            reauthenticator.clone(),
        ),
        delete_account_usecase: DeleteAccount::new(
            user_repository.clone(),
            unit_of_work.clone(),
            access_revoker.clone(),
            reauthenticator.clone(),
        ),
        user_repository: user_repository.clone(),
        trust_forwarded_headers: server_config.trust_forwarded_headers,
    });

    let rate_limiter =
        api_common::rate_limiting::create_rate_limiter_from_config("api", &rate_limiting_config)
            .await
            .ok_or("Failed to create rate limiter: invalid parameters")?;
    let rate_limiter_data = web::Data::new(rate_limiter);

    let readiness = Readiness::builder()
        .add_probe({
            let pool = postgres_pool.clone();
            move || {
                let pool = pool.clone();
                async move { ping_pool(&pool).await }
            }
        })
        .build();
    readiness.mark_ready();
    let readiness_data = web::Data::new(readiness);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .expose_headers(["x-request-id"])
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .app_data(readiness_data.clone())
            .app_data(rate_limiter_data.clone())
            .app_data(access_revocations_data.clone())
            .wrap(RequestIdMiddleware)
            .wrap(Logger::default())
            .wrap(cors)
            .configure(routes::configure)
    })
    .bind((server_config.host, server_config.port))?
    .run()
    .await?;

    Ok(())
}
