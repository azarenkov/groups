use std::{error::Error, sync::Arc};
use api_common::shutdown::{self, ShutdownConfig};

use actix_web::{App, HttpServer, middleware::Logger, web};
use voice::usecases::{
    call::{
        accept_call::AcceptCall, add_participants::AddCallParticipants,
        decline_call::DeclineCall, get_active_call::GetActiveCall,
        get_active_call_with::GetActiveCallWith, join_call::JoinCall, leave_call::LeaveCall,
        start_call::StartCall,
    },
    webhook::voice::HandleVoiceWebhook,
};
use infrastructure_common::{
    config::{livekit::LiveKitConfig, postgres::PostgresConfig},
};
use voice_infrastructure::{
    media_client::friendship::MessagingFriendshipClient,
    outbox::webhook_outbox::PostgresWebhookOutbox,
    database::postgres::{
        MIGRATOR, SCHEMA,
        connection::{create_postgres_pool, ping_pool},
        repositories::{
            call::PostgresCallRepository, media_state::PostgresMediaStateRepository,
            outbox_writer::PostgresOutboxWriter,
            user_profile::PostgresUserProfileRepository,
        },
    },
    voice::{
        livekit_service::{LiveKitService, is_livekit_reachable},
        livekit_webhook_receiver::LiveKitWebhookReceiver,
    },
};
use api_common::{
    health::Readiness,
    middleware::request_id::RequestIdMiddleware,
    rate_limiting::config::RateLimitingConfig,
};

use api_common::config::ServerConfig;

use crate::{routes, state::AppState};
use infrastructure_common::messaging::nats::connection::{stream_name, streams};

pub async fn run() -> Result<(), Box<dyn Error>> {
    infrastructure_common::observability::init("voice-service");

    let server_config = ServerConfig::from_env()?;
    let postgres_config = PostgresConfig::from_env_for(SCHEMA)?;
    let livekit_config = LiveKitConfig::from_env()?;
    let rate_limiting_config = RateLimitingConfig::from_env()?;

    let postgres_pool = create_postgres_pool(postgres_config, SCHEMA, Some(&MIGRATOR)).await?;
    let livekit_url = livekit_config.url().to_string();

    let call_repository = Arc::new(PostgresCallRepository::new(postgres_pool.clone()));
    let webhook_outbox = Arc::new(PostgresWebhookOutbox::new(PostgresOutboxWriter::new(
        postgres_pool.clone(),
    )));
    let user_repository = Arc::new(PostgresUserProfileRepository::voice(postgres_pool.clone()));
    let plan_directory = Arc::new(
        infrastructure_common::database::postgres::repositories::plan_directory::PostgresPlanDirectory::voice(
            postgres_pool.clone(),
        ),
    );
    let messaging_internal_url = api_common::internal_client::InternalService::Messaging.base_url();
    let friendship_lookup = Arc::new(MessagingFriendshipClient::new(&messaging_internal_url));

    let nats_config = infrastructure_common::config::nats::NatsConfig::from_env()?;
    let nats_jetstream =
        infrastructure_common::messaging::nats::connection::create_jetstream(&nats_config).await?;
    let user_stream = stream_name("VOICE_USER_STREAM", streams::USER_EVENTS);
    let livekit_webhook_receiver = Arc::new(LiveKitWebhookReceiver::new(&livekit_config));
    let livekit_service = Arc::new(LiveKitService::new(livekit_config));
    crate::evictions::spawn(call_repository.clone(), livekit_service.clone());
    crate::call_sweep::spawn(call_repository.clone(), livekit_service.clone());
    let media_state_repository = Arc::new(PostgresMediaStateRepository::new(postgres_pool.clone()));
    crate::stream_sweep::spawn(Arc::new(voice::usecases::stream::sweep_screen_shares::SweepScreenShares::new(
        plan_directory.clone(),
        livekit_service.clone(),
        webhook_outbox.clone(),
    )));
    infrastructure_common::messaging::nats::plan_projection::PlanProjection {
        jetstream: nats_jetstream.clone(),
        subject_prefix: nats_config.subject_prefix.clone(),
        stream: user_stream.clone(),
        consumer_name: "voice-projection-tiers",
        plans: plan_directory.clone(),
        listener: Arc::new(voice::usecases::stream::refresh_stream_ceilings::RefreshStreamCeilings::new(
            plan_directory.clone(),
            livekit_service.clone(),
        )),
    }
    .spawn();
    crate::projection::spawn(
        nats_jetstream,
        nats_config.subject_prefix.clone(),
        user_stream,
        user_repository.clone(),
        Arc::new(
            voice::usecases::call::enforce_user_deactivation::EnforceUserDeactivation::new(
                call_repository.clone(),
                livekit_service.clone(),
            ),
        ),
        Arc::new(voice::usecases::call::enforce_block::EnforceBlock::new(
            call_repository.clone(),
            livekit_service.clone(),
            friendship_lookup.clone(),
        )),
    );

    let app_state = web::Data::new(AppState {
        start_call_usecase: StartCall::new(
            call_repository.clone(),
            livekit_service.clone(),
            friendship_lookup.clone(),
            user_repository.clone(),
        ),
        accept_call_usecase: AcceptCall::new(
            call_repository.clone(),
            livekit_service.clone(),
            friendship_lookup.clone(),
            user_repository.clone(),
        ),
        decline_call_usecase: DeclineCall::new(call_repository.clone()),
        leave_call_usecase: LeaveCall::new(call_repository.clone()),
        add_call_participants_usecase: AddCallParticipants::new(
            call_repository.clone(),
            livekit_service.clone(),
            friendship_lookup.clone(),
            user_repository.clone(),
        ),
        get_active_call_usecase: GetActiveCall::new(
            call_repository.clone(),
            livekit_service.clone(),
            friendship_lookup.clone(),
            user_repository.clone(),
        ),
        get_active_call_with_usecase: GetActiveCallWith::new(
            call_repository.clone(),
            friendship_lookup.clone(),
        ),
        join_call_usecase: JoinCall::new(
            call_repository.clone(),
            livekit_service.clone(),
            friendship_lookup.clone(),
            user_repository.clone(),
        ),
        handle_livekit_webhook_usecase: HandleVoiceWebhook::new(
            livekit_webhook_receiver.clone(),
            call_repository.clone(),
            webhook_outbox.clone(),
            user_repository.clone(),
            livekit_service.clone(),
            friendship_lookup.clone(),
            plan_directory.clone(),
        ),
        get_media_state_usecase: voice::usecases::media::get_media_state::GetMediaState::new(
            media_state_repository.clone(),
        ),
        save_media_state_usecase: voice::usecases::media::save_media_state::SaveMediaState::new(
            media_state_repository,
        ),
        call_repository: call_repository.clone(),
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
        .add_probe({
            let url = livekit_url.clone();
            move || {
                let url = url.clone();
                async move { is_livekit_reachable(&url).await }
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
