use std::sync::Arc;

use contracts::events::{channel::ParticipantMutedV1, channel::ParticipantUnmutedV1, server::ServerMemberV1, user::UserLifecycleV1};
use domain::user::id::UserId;
use futures::StreamExt;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use messaging::{
    ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions},
    ports::voice_service::VoiceService,
    usecases::channel::enforce_user_deactivation::EnforceUserDeactivation,
    usecases::channel::enforce_voice_mute::EnforceVoiceMute,
    usecases::server::enforce_departure::EnforceServerDeparture,
    usecases::server::resolve_deleted_user_servers::ResolveDeletedUserServers,
};
use messaging_domain::{server::repository::ServerRepository, channel::id::ChannelId, channel::repository::ChannelRepository, server::id::ServerId, server::unit_of_work::ServerUnitOfWork};
use ports::user_profile::UserProfileRepository;
use tracing::Instrument;

const MAX_DELIVER: u32 = 20;
const ACK_WAIT: std::time::Duration = std::time::Duration::from_secs(300);
const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

pub struct VoiceEffectsSpec {
    pub subject_prefix: String,
    pub channel_stream: String,
    pub server_stream: String,
    pub user_stream: String,
}

pub fn spawn<C, W, V, P, S>(
    jetstream: async_nats::jetstream::Context,
    spec: VoiceEffectsSpec,
    channel_repository: Arc<C>,
    server_unit_of_work: Arc<W>,
    voice_service: Arc<V>,
    profiles: Arc<P>,
    server_repository: Arc<S>,
) where
    C: ChannelRepository + Send + Sync + 'static,
    W: ServerUnitOfWork + Send + Sync + 'static,
    V: VoiceService + Send + Sync + 'static,
    P: UserProfileRepository + Send + Sync + 'static,
    S: ServerRepository + Send + Sync + 'static,
{
    let prefix = spec.subject_prefix.trim_end_matches('.').to_string();
    let dlq = format!("{prefix}.dlq.messaging.voice_effects");
    let enforce_mute = Arc::new(EnforceVoiceMute::new(
        channel_repository.clone(),
        server_unit_of_work.clone(),
        voice_service.clone(),
    ));
    let enforce_deactivation = Arc::new(EnforceUserDeactivation::new(
        channel_repository.clone(),
        voice_service.clone(),
        profiles,
    ));
    let resolve_servers = Arc::new(ResolveDeletedUserServers::new(
        server_repository,
        channel_repository.clone(),
        server_unit_of_work.clone(),
        voice_service.clone(),
    ));
    let enforce_departure = Arc::new(EnforceServerDeparture::new(
        server_unit_of_work,
        channel_repository,
        voice_service,
    ));

    {
        let jetstream = jetstream.clone();
        let subject = format!("{prefix}.user.*.disabled");
        let extra = vec![
            format!("{prefix}.user.*.deleted"),
            format!("{prefix}.user.disabled"),
            format!("{prefix}.user.deleted"),
        ];
        let dlq = dlq.clone();
        let user_stream = spec.user_stream.clone();
        tokio::spawn(async move {
            run(
                jetstream,
                user_stream,
                "messaging-voice-effects-user".to_string(),
                subject,
                extra,
                dlq,
                move |subject, payload| {
                    let enforce_deactivation = enforce_deactivation.clone();
                    let resolve_servers = resolve_servers.clone();
                    async move {
                        if !(subject.ends_with(".disabled") || subject.ends_with(".deleted")) {
                            return Ok(());
                        }
                        let evt: UserLifecycleV1 = serde_json::from_slice(&payload)?;
                        enforce_deactivation
                            .execute(UserId::new(evt.user_id), evt.occurred_at)
                            .await?;
                        if subject.ends_with(".deleted") {
                            resolve_servers
                                .execute(UserId::new(evt.user_id), evt.occurred_at, None)
                                .await?;
                        }
                        Ok(())
                    }
                },
            )
            .await;
        });
    }

    {
        let jetstream = jetstream.clone();
        let subject = format!("{prefix}.channel.*.participant.muted");
        let extra = vec![
            format!("{prefix}.channel.*.participant.unmuted"),
            format!("{prefix}.channel.participant.muted"),
            format!("{prefix}.channel.participant.unmuted"),
        ];
        let dlq = dlq.clone();
        tokio::spawn(async move {
            run(
                jetstream,
                spec.channel_stream,
                "messaging-voice-effects-channel".to_string(),
                subject,
                extra,
                dlq,
                move |subject, payload| {
                    let enforce_mute = enforce_mute.clone();
                    async move {
                        let (channel_id, user_id) = if subject.ends_with(".participant.muted") {
                            let evt: ParticipantMutedV1 = serde_json::from_slice(&payload)?;
                            (evt.channel_id, evt.user_id)
                        } else if subject.ends_with(".participant.unmuted") {
                            let evt: ParticipantUnmutedV1 = serde_json::from_slice(&payload)?;
                            (evt.channel_id, evt.user_id)
                        } else {
                            return Ok(());
                        };
                        enforce_mute
                            .execute(ChannelId::new(channel_id), UserId::new(user_id))
                            .await?;
                        Ok(())
                    }
                },
            )
            .await;
        });
    }
    {
        let subject = format!("{prefix}.server.*.member_left");
        tokio::spawn(async move {
            run(
                jetstream,
                spec.server_stream,
                "messaging-voice-effects-server".to_string(),
                subject,
                vec![format!("{prefix}.server.member_left")],
                dlq,
                move |subject, payload| {
                    let enforce_departure = enforce_departure.clone();
                    async move {
                        if !subject.ends_with(".member_left") {
                            return Ok(());
                        }
                        let evt: ServerMemberV1 = serde_json::from_slice(&payload)?;
                        enforce_departure
                            .execute(ServerId::new(evt.server_id), UserId::new(evt.user_id))
                            .await?;
                        Ok(())
                    }
                },
            )
            .await;
        });
    }
}

async fn run<F, Fut>(
    jetstream: async_nats::jetstream::Context,
    stream: String,
    consumer_name: String,
    subject: String,
    extra_subjects: Vec<String>,
    dlq: String,
    handler: F,
) where
    F: Fn(String, Vec<u8>) -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
{
    let consumer = NatsEventConsumer::new(jetstream, stream.clone());
    loop {
        let opts = SubscribeOptions {
            consumer_name: consumer_name.clone(),
            subject: subject.clone(),
            start_position: StartPosition::All,
            max_deliver: Some(MAX_DELIVER),
            dlq_subject: Some(dlq.clone()),
            extra_subjects: extra_subjects.clone(),
            ack_wait: Some(ACK_WAIT),
        };
        match consumer.subscribe(opts).await {
            Ok(mut messages) => {
                tracing::info!(%stream, consumer = %consumer_name, "Voice effects consumer subscribed");
                while let Some(msg) = messages.next().await {
                    match msg {
                        Ok(m) => {
                            let span = tracing::info_span!(
                                "voice_effect",
                                request_id = ?m.request_id,
                                subject = %m.subject,
                            );
                            async {
                                match handler(m.subject.clone(), m.payload.to_vec()).await {
                                    Ok(()) => {
                                        if let Err(e) = m.ack().await {
                                            tracing::warn!(error = %e, "Voice effect ack failed");
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, subject = %m.subject, "Voice effect failed; scheduling retry");
                                        if let Err(e) = m.retry_after(RETRY_DELAY).await {
                                            tracing::warn!(error = %e, "Voice effect nak failed");
                                        }
                                    }
                                }
                            }
                            .instrument(span)
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Voice effects stream error");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, %stream, "Voice effects subscribe failed; retrying");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
