use identity::usecases::entitlements::{
    get_entitlements::GetEntitlements, grant_entitlements::GrantEntitlements,
    set_badge_style::SetBadgeStyle,
};
use identity::usecases::user::activity::{ClearActivity, GetActivities, SetActivity};
use identity_infrastructure::database::postgres::repositories::entitlements::PostgresEntitlementsRepository;
use api_common::shutdown::{self, ShutdownConfig};
use identity_infrastructure::activity::redis_activity_store::RedisActivityStore;
use identity_infrastructure::media_client::activity_audience::MessagingAudienceClient;
use identity_infrastructure::media_client::block_lookup::MessagingBlockClient;
use std::{error::Error, sync::Arc};

use actix_web::{App, HttpServer, middleware::Logger, web};
use identity::usecases::{
    connections::{
        complete_connection::CompleteConnection, list_connections::ListConnections,
        list_user_connections::ListUserConnections,
        remove_connection::RemoveConnection, start_connection::StartConnection,
    },
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
        get_me::GetMe, rename::RenameUser, search_users::SearchUsers,
        update_locale::UpdateUserLocale,
        change_password::ChangePassword, confirm_email_change::ConfirmEmailChange,
        delete_account::DeleteAccount, disable_account::DisableAccount,
        purge_unverified_accounts::PurgeUnverifiedAccounts,
        reauth::Reauthenticator, request_email_change::RequestEmailChange,
        update_profile::UpdateProfile,
    },
};
use domain::user::name::ReservedNames;
use infrastructure_common::{
    config::{
        nsfw::NsfwConfig, postgres::PostgresConfig,
        trial::TrialConfig, unverified_cleanup::UnverifiedCleanupConfig, username::UsernameConfig,
    },
    hasher::bcrypt_hasher::BcryptHasher,
};
use identity::usecases::auth::captcha::CaptchaGate;
use identity::usecases::entitlements::{expire_entitlements::ExpireEntitlements, remind_trial_ending::RemindTrialEnding, start_trial::StartTrial};
use identity_infrastructure::auth::{altcha::{AltchaConfig, AltchaVerifier}, redis_captcha_store::RedisCaptchaStore};
use identity_infrastructure::{
    auth::{github_connector::GithubOAuthConnector, jwt_token_issuer::JwtTokenIssuer, redis_access_revoker::RedisAccessRevoker, steam_connector::SteamOpenIdConnector},
    database::postgres::{
        MIGRATOR, SCHEMA,
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            identity_unit_of_work::PostgresIdentityUnitOfWork,
            user::PostgresUserRepository,
        },
    },
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
    let postgres_config = PostgresConfig::from_env_for(SCHEMA)?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;
    let altcha_config = AltchaConfig::from_env()?;
    let nsfw_config = NsfwConfig::from_env()?;
    let username_config = UsernameConfig::from_env()?;
    let unverified_cleanup_config = UnverifiedCleanupConfig::from_env()?;
    let trial_config = TrialConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config, SCHEMA, Some(&MIGRATOR)).await?;

    let user_repository = Arc::new(PostgresUserRepository::new(postgres_pool.clone()));
    let trial_hash_key = std::env::var("TRIAL_HASH_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
        .unwrap_or_else(|| {
            tracing::warn!("TRIAL_HASH_KEY is not set; trial grant hashes fall back to JWT_SECRET and reset when it rotates");
            server_config.jwt_secret.clone()
        });
    let unit_of_work = Arc::new(PostgresIdentityUnitOfWork::new(
        postgres_pool.clone(),
        trial_hash_key.as_bytes(),
    ));
    let entitlements_repository =
        Arc::new(PostgresEntitlementsRepository::new(postgres_pool.clone()));

    let hasher = Arc::new(BcryptHasher::default());
    let token_issuer = Arc::new(JwtTokenIssuer::new(server_config.jwt_secret.clone()));
    let nsfw_detector = Arc::new(NsfwDetector::new(&nsfw_config));
    let reserved_names = Arc::new(ReservedNames::new(username_config.extra_reserved()));

    let github_connector = Arc::new(GithubOAuthConnector::new(
        std::env::var("GITHUB_CLIENT_ID").unwrap_or_default(),
        std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default(),
    ));
    let steam_connector = Arc::new(SteamOpenIdConnector::new(
        std::env::var("STEAM_API_KEY").unwrap_or_default(),
    ));
    let public_api_url =
        std::env::var("PUBLIC_API_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let public_app_url = std::env::var("PUBLIC_APP_URL").unwrap_or_default();

    let access_revocations = api_common::middleware::revocation::AccessRevocations::from_env().await;
    let access_revocations_data = web::Data::new(access_revocations.clone());
    let access_revoker = Arc::new(RedisAccessRevoker::new(access_revocations));
    let activity_store = Arc::new(RedisActivityStore::from_env());
    let messaging_internal_url = std::env::var("MESSAGING_INTERNAL_URL")
        .unwrap_or_else(|_| "http://messaging-service:8080".to_string());
    let activity_audience = Arc::new(MessagingAudienceClient::new(&messaging_internal_url));
    let block_lookup = Arc::new(MessagingBlockClient::new(&messaging_internal_url));
    if unverified_cleanup_config.enabled() {
        let purge_unverified = Arc::new(PurgeUnverifiedAccounts::new(
            unit_of_work.clone(),
            access_revoker.clone(),
            activity_store.clone(),
            unverified_cleanup_config.ttl(),
            unverified_cleanup_config.batch_size(),
        ));
        let sweep_interval = unverified_cleanup_config.interval();
        actix_web::rt::spawn(async move {
            let mut ticker = actix_web::rt::time::interval(sweep_interval);
            loop {
                ticker.tick().await;
                match purge_unverified.run_once().await {
                    Ok(0) => {}
                    Ok(purged) => {
                        tracing::info!(purged, "Removed unverified accounts past their lifetime")
                    }
                    Err(e) => tracing::warn!(error = %e, "Unverified account cleanup failed"),
                }
            }
        });
    }

    let expire_entitlements = Arc::new(ExpireEntitlements::new(
        unit_of_work.clone(),
        trial_config.expiry_batch_size(),
    ));
    let remind_trial_ending = Arc::new(RemindTrialEnding::new(
        unit_of_work.clone(),
        user_repository.clone(),
        trial_config.expiry_batch_size(),
    ));
    let expiry_interval = trial_config.expiry_interval();
    actix_web::rt::spawn(async move {
        let mut ticker = actix_web::rt::time::interval(expiry_interval);
        loop {
            ticker.tick().await;
            match expire_entitlements.drain().await {
                Ok(0) => {}
                Ok(expired) => tracing::info!(expired, "Moved lapsed plans back to Free"),
                Err(e) => tracing::warn!(error = %e, "Plan expiry sweep failed"),
            }
            match remind_trial_ending.drain().await {
                Ok(0) => {}
                Ok(reminded) => tracing::info!(reminded, "Sent trial ending reminders"),
                Err(e) => tracing::warn!(error = %e, "Trial reminder sweep failed"),
            }
        }
    });

    let reauthenticator = Arc::new(Reauthenticator::new(hasher.clone()));

    let captcha_gate = CaptchaGate::new(
        Arc::new(AltchaVerifier::from_config(&altcha_config)),
        Arc::new(RedisCaptchaStore::from_env(std::time::Duration::from_secs(
            altcha_config.altcha_login_window_secs,
        ))),
        altcha_config.altcha_login_failures,
    );

    let app_state = web::Data::new(AppState {
        captcha_gate,
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
        request_password_reset_usecase: RequestPasswordReset::new(
            user_repository.clone(),
            unit_of_work.clone(),
            hasher.clone(),
        ),
        reset_password_usecase: ResetPassword::new(unit_of_work.clone(), hasher.clone(), access_revoker.clone()),
        login_user_usecase: LoginUser::new(
            user_repository.clone(),
            hasher.clone(),
            unit_of_work.clone(),
            token_issuer.clone(),
        ),
        confirm_registration_usecase: ConfirmRegistration::new(hasher.clone(), unit_of_work.clone()),
        refresh_tokens_usecase: RefreshTokens::new(unit_of_work.clone(), token_issuer.clone()),
        logout_usecase: Logout::new(unit_of_work.clone(), token_issuer.clone()),
        search_users_usecase: SearchUsers::new(user_repository.clone()),
        get_me_usecase: GetMe::new(user_repository.clone()),
        update_user_locale_usecase: UpdateUserLocale::new(unit_of_work.clone()),
        update_profile_usecase: UpdateProfile::new(
            unit_of_work.clone(),
            activity_store.clone(),
            nsfw_detector.clone(),
        ),
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
        confirm_email_change_usecase: ConfirmEmailChange::new(
            unit_of_work.clone(),
            hasher.clone(),
            token_issuer.clone(),
            access_revoker.clone(),
        ),
        disable_account_usecase: DisableAccount::new(
            user_repository.clone(),
            unit_of_work.clone(),
            access_revoker.clone(),
            reauthenticator.clone(),
            activity_store.clone(),
        ),
        delete_account_usecase: DeleteAccount::new(
            user_repository.clone(),
            unit_of_work.clone(),
            access_revoker.clone(),
            reauthenticator.clone(),
            activity_store.clone(),
        ),
        get_entitlements_usecase: GetEntitlements::new(entitlements_repository.clone()),
        grant_entitlements_usecase: GrantEntitlements::new(unit_of_work.clone()),
        start_trial_usecase: StartTrial::new(
            unit_of_work.clone(),
            trial_config.ip_limit(),
            trial_config.ip_window(),
        ),
        set_badge_style_usecase: SetBadgeStyle::new(unit_of_work.clone()),
        get_appearance_usecase: GetAppearance::new(user_repository.clone()),
        update_appearance_usecase: UpdateAppearance::new(unit_of_work.clone()),
        set_activity_usecase: SetActivity::new(activity_store.clone(), unit_of_work.clone()),
        clear_activity_usecase: ClearActivity::new(activity_store.clone(), unit_of_work.clone()),
        activity_store: activity_store.clone(),
        get_activities_usecase: GetActivities::new(activity_store.clone(), activity_audience.clone()),
        get_voice_settings_usecase: GetVoiceSettings::new(user_repository.clone(), entitlements_repository.clone()),
        update_voice_settings_usecase: UpdateVoiceSettings::new(unit_of_work.clone()),
        get_notification_settings_usecase: GetNotificationSettings::new(user_repository.clone()),
        update_notification_settings_usecase: UpdateNotificationSettings::new(unit_of_work.clone()),
        get_hotkey_settings_usecase: GetHotkeySettings::new(user_repository.clone()),
        update_hotkey_settings_usecase: UpdateHotkeySettings::new(unit_of_work.clone()),
        start_connection_usecase: StartConnection::new(
            user_repository.clone(),
            github_connector.clone(),
            steam_connector.clone(),
            public_api_url.clone(),
        ),
        complete_connection_usecase: CompleteConnection::new(
            user_repository.clone(),
            github_connector.clone(),
            steam_connector.clone(),
            public_api_url.clone(),
        ),
        list_connections_usecase: ListConnections::new(user_repository.clone()),
        list_user_connections_usecase: ListUserConnections::new(
            user_repository.clone(),
            block_lookup.clone(),
        ),
        remove_connection_usecase: RemoveConnection::new(user_repository.clone()),
        public_app_url: public_app_url.clone(),
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
    let readiness_data = web::Data::new(readiness.clone());
    let shutdown_config = ShutdownConfig::from_env();

    let server = HttpServer::new(move || {
        let cors = api_common::cors::cors();

        App::new()
            .wrap(actix_web::middleware::from_fn(
                api_common::middleware::internal::guard_internal_routes,
            ))
            .app_data(app_state.clone())
            .app_data(readiness_data.clone())
            .app_data(rate_limiter_data.clone())
            .app_data(access_revocations_data.clone())
            .wrap(RequestIdMiddleware)
            .wrap(Logger::default().exclude("/readyz").exclude("/livez"))
            .wrap(cors)
            .configure(routes::configure)
    })
    .disable_signals()
    .shutdown_timeout(shutdown_config.shutdown_timeout.as_secs())
    .bind((server_config.host, server_config.port))?
    .run();
    shutdown::serve(server, readiness, shutdown_config).await?;

    Ok(())
}
