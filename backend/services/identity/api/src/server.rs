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
    },
};
use domain::user::name::ReservedNames;
use platform::{
    config::{
        google::GoogleOAuthConfig, nsfw::NsfwConfig, postgres::PostgresConfig,
        username::UsernameConfig,
    },
    hasher::bcrypt_hasher::BcryptHasher,
};
use identity_infrastructure::{
    auth::{google_verifier::GoogleIdTokenVerifier, jwt_token_issuer::JwtTokenIssuer},
    database::postgres::{
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            refresh_token::PostgresRefreshTokenRepository, user::PostgresUserRepository,
            verification_code::PostgresVerificationCodeRepository,
        },
    },
    media_client::user_state::MessagingStateClient,
    moderation::nsfw_detector::NsfwDetector,
};
use shared::{
    health::Readiness,
    middleware::request_id::RequestIdMiddleware,
    rate_limiting::config::RateLimitingConfig,
};

use api_common::config::ServerConfig;

use crate::{routes, state::AppState};

pub async fn run() -> Result<(), Box<dyn Error>> {
    shared::observability::init("identity-service");

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env()?;
    let google_config = GoogleOAuthConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;
    let nsfw_config = NsfwConfig::from_env()?;
    let username_config = UsernameConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config).await?;

    let user_repository = Arc::new(PostgresUserRepository::new(postgres_pool.clone()));
    let verification_code_repository = Arc::new(PostgresVerificationCodeRepository::new(
        postgres_pool.clone(),
    ));
    let refresh_token_repository =
        Arc::new(PostgresRefreshTokenRepository::new(postgres_pool.clone()));

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

    let app_state = web::Data::new(AppState {
        register_user_usecase: RegisterUser::new(
            user_repository.clone(),
            hasher.clone(),
            token_issuer.clone(),
            nsfw_detector.clone(),
            reserved_names.clone(),
        ),
        issue_verification_code_usecase: IssueVerificationCode::new(
            user_repository.clone(),
            verification_code_repository.clone(),
            hasher.clone(),
        ),
        login_user_usecase: LoginUser::new(
            user_repository.clone(),
            hasher.clone(),
            refresh_token_repository.clone(),
            token_issuer.clone(),
        ),
        google_login_usecase: GoogleLogin::new(
            user_repository.clone(),
            refresh_token_repository.clone(),
            token_issuer.clone(),
            google_token_verifier.clone(),
            hasher.clone(),
            nsfw_detector.clone(),
            reserved_names.clone(),
            user_state_lookup.clone(),
        ),
        confirm_registration_usecase: ConfirmRegistration::new(
            user_repository.clone(),
            hasher.clone(),
            verification_code_repository.clone(),
        ),
        refresh_tokens_usecase: RefreshTokens::new(
            refresh_token_repository.clone(),
            user_repository.clone(),
            token_issuer.clone(),
        ),
        logout_usecase: Logout::new(refresh_token_repository.clone(), token_issuer.clone()),
        search_users_usecase: SearchUsers::new(user_repository.clone()),
        get_me_usecase: GetMe::new(user_repository.clone()),
        update_user_locale_usecase: UpdateUserLocale::new(user_repository.clone()),
        rename_user_usecase: RenameUser::new(
            user_repository.clone(),
            nsfw_detector.clone(),
            reserved_names.clone(),
        ),
        user_repository: user_repository.clone(),
        trust_forwarded_headers: server_config.trust_forwarded_headers,
    });

    let rate_limiter =
        shared::rate_limiting::create_rate_limiter_from_config("api", &rate_limiting_config)
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
