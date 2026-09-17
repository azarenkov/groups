use std::{error::Error, sync::Arc};
use api_common::shutdown::{self, ShutdownConfig};

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};
use media::usecases::{
    attachment::{
        cleanup_orphans::CleanupOrphanAttachments, delete_attachment::DeleteAttachment,
        upload_attachment::UploadAttachment,
    },
    server::{get_logo::GetServerLogo, update_logo::UpdateServerLogo},
    user::{
        delete_avatar::DeleteUserAvatar, get_avatar::GetUserAvatar,
        update_avatar::UpdateUserAvatar,
    },
};
use infrastructure_common::{
    config::{minio::MinioConfig, nsfw::NsfwConfig, postgres::PostgresConfig},
    storage::minio_storage::MinioStorage,
};
use media_infrastructure::{
    database::postgres::{
        MIGRATOR, SCHEMA,
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            attachment::PostgresAttachmentRepository, media::PostgresMediaRepository,
        },
    },
    media_client::{links::MessagingLinksClient, owner_lookup::MessagingOwnerClient},
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
    infrastructure_common::observability::init("media-service");

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env_for(SCHEMA)?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;
    let minio_config = MinioConfig::from_env()?;
    let nsfw_config = NsfwConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config, SCHEMA, Some(&MIGRATOR)).await?;

    let media_repository = Arc::new(PostgresMediaRepository::new(postgres_pool.clone()));
    let banner_repository = Arc::new(
        media_infrastructure::database::postgres::repositories::banner::PostgresBannerRepository::new(
            postgres_pool.clone(),
        ),
    );
    let attachment_repository =
        Arc::new(PostgresAttachmentRepository::new(postgres_pool.clone()));

    let minio_storage = Arc::new(
        MinioStorage::new(&minio_config)
            .map_err(|e| Box::<dyn Error>::from(format!("minio init failed: {e}")))?,
    );
    if let Err(e) = minio_storage.ensure_bucket(&minio_config).await {
        tracing::warn!(error = %e, "Failed to ensure MinIO bucket; retrying in background");
        let retry_storage = minio_storage.clone();
        let retry_config = minio_config.clone();
        actix_web::rt::spawn(async move {
            let mut delay = std::time::Duration::from_secs(2);
            loop {
                actix_web::rt::time::sleep(delay).await;
                match retry_storage.ensure_bucket(&retry_config).await {
                    Ok(()) => {
                        tracing::info!("MinIO bucket ensured after retry");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "MinIO bucket ensure retry failed");
                        delay = (delay * 2).min(std::time::Duration::from_secs(60));
                    }
                }
            }
        });
    }

    let messaging_internal_url = std::env::var("MESSAGING_INTERNAL_URL")
        .unwrap_or_else(|_| "http://messaging-service:8080".to_string());
    let links_lookup = Arc::new(MessagingLinksClient::new(&messaging_internal_url));
    let orphan_cleanup = Arc::new(CleanupOrphanAttachments::new(
        attachment_repository.clone(),
        minio_storage.clone(),
        links_lookup.clone(),
    ));
    {
        let orphan_cleanup = orphan_cleanup.clone();
        actix_web::rt::spawn(async move {
            let mut interval =
                actix_web::rt::time::interval(std::time::Duration::from_secs(600));
            loop {
                interval.tick().await;
                orphan_cleanup.run_once().await;
            }
        });
    }

    let nats_config = infrastructure_common::config::nats::NatsConfig::from_env()?;
    let nats_jetstream =
        infrastructure_common::messaging::nats::connection::create_jetstream(&nats_config).await?;
    let server_stream = std::env::var("MEDIA_SERVER_STREAM")
        .unwrap_or_else(|_| "RIFT_SERVER_EVENTS".to_string());
    crate::projection::spawn(
        nats_jetstream.clone(),
        nats_config.subject_prefix.clone(),
        server_stream,
        media_repository.clone(),
        minio_storage.clone(),
    );
    let storage_stream = std::env::var("MEDIA_STORAGE_STREAM")
        .unwrap_or_else(|_| "RIFT_STORAGE_EVENTS".to_string());
    crate::storage_release::spawn(
        nats_jetstream.clone(),
        nats_config.subject_prefix.clone(),
        storage_stream,
        attachment_repository.clone(),
    );
    let user_stream = std::env::var("MEDIA_USER_STREAM")
        .unwrap_or_else(|_| "RIFT_USER_EVENTS".to_string());
    crate::user_projection::spawn(
        nats_jetstream,
        nats_config.subject_prefix.clone(),
        user_stream,
        media_repository.clone(),
        banner_repository.clone(),
        minio_storage.clone(),
    );

    let owner_lookup = Arc::new(MessagingOwnerClient::new(&messaging_internal_url));
    let nsfw_detector = Arc::new(NsfwDetector::new(&nsfw_config));
    let upload_semaphore = Arc::new(tokio::sync::Semaphore::new(8));
    let image_read_limiter = api_common::rate_limiting::create_limiter("image-read", 60, 400)
        .await
        .ok_or("Failed to create image read limiter")?;

    let app_state = web::Data::new(AppState {
        upload_attachment_usecase: UploadAttachment::new(
            attachment_repository.clone(),
            minio_storage.clone(),
            banner_repository.clone(),
        ),
        upload_semaphore,
        image_read_limiter,
        delete_attachment_usecase: DeleteAttachment::new(
            attachment_repository.clone(),
            minio_storage.clone(),
        ),
        attachment_repository: attachment_repository.clone(),
        update_user_avatar_usecase: UpdateUserAvatar::new(
            media_repository.clone(),
            attachment_repository.clone(),
            minio_storage.clone(),
            nsfw_detector.clone(),
        ),
        delete_user_avatar_usecase: DeleteUserAvatar::new(
            media_repository.clone(),
            attachment_repository.clone(),
            minio_storage.clone(),
        ),
        get_user_avatar_usecase: GetUserAvatar::new(
            media_repository.clone(),
            minio_storage.clone(),
        ),
        update_user_banner_usecase: media::usecases::user::banner::UpdateUserBanner::new(
            banner_repository.clone(),
            attachment_repository.clone(),
            minio_storage.clone(),
            nsfw_detector.clone(),
        ),
        get_user_banner_usecase: media::usecases::user::banner::GetUserBanner::new(
            banner_repository.clone(),
            minio_storage.clone(),
        ),
        update_server_logo_usecase: UpdateServerLogo::new(
            media_repository.clone(),
            attachment_repository.clone(),
            minio_storage.clone(),
            nsfw_detector.clone(),
            owner_lookup.clone(),
        ),
        get_server_logo_usecase: GetServerLogo::new(
            media_repository.clone(),
            minio_storage.clone(),
        ),
    });

    let rate_limiter =
        api_common::rate_limiting::create_rate_limiter_from_config("api", &rate_limiting_config)
            .await
            .ok_or("Failed to create rate limiter: invalid parameters")?;
    let rate_limiter_data = web::Data::new(rate_limiter);
    let access_revocations_data =
        web::Data::new(api_common::middleware::revocation::AccessRevocations::from_env().await);

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
