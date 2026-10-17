use std::sync::Arc;

use contracts::events::{call::CallEventV1, channel::ParticipantDeafenedV1, channel::ParticipantMutedV1, channel::ParticipantUndeafenedV1, channel::ParticipantUnmutedV1, community::CommunityMemberV1, user::UserLifecycleV1};
use domain::user::id::UserId;
use infrastructure_common::messaging::nats::runner::{ConsumerSpec, Delivery as RunnerDelivery, Event, Lifecycle, dlq_subject, run_consumer};
use messaging::{
    ports::event_consumer::StartPosition,
    ports::call_participation::CallParticipationLookup,
    ports::voice_service::VoiceService,
    usecases::call::log_call::LogCall,
    usecases::channel::enforce_user_deactivation::EnforceUserDeactivation,
    usecases::channel::enforce_voice_mute::EnforceVoiceMute,
    usecases::community::enforce_departure::EnforceCommunityDeparture,
    usecases::community::resolve_deleted_user_communities::ResolveDeletedUserCommunities,
};
use messaging_domain::{dm_message::unit_of_work::DmUnitOfWork, channel::id::ChannelId, channel::repository::ChannelRepository, community::id::CommunityId, community::unit_of_work::CommunityUnitOfWork};
use ports::user_profile::UserProfileRepository;

const ACK_WAIT: std::time::Duration = std::time::Duration::from_secs(300);

#[derive(Clone, Copy)]
struct Delivery {
    start: StartPosition,
    max_deliver: u32,
    retry_delay: std::time::Duration,
}

const EFFECT_DELIVERY: Delivery = Delivery { start: StartPosition::All, max_deliver: 20, retry_delay: std::time::Duration::from_secs(5) };

fn call_log_delivery(since: chrono::DateTime<chrono::Utc>) -> Delivery {
    Delivery { start: StartPosition::Since(since), max_deliver: 120, retry_delay: std::time::Duration::from_secs(30) }
}

pub struct VoiceEffectsSpec {
    pub subject_prefix: String,
    pub channel_stream: String,
    pub community_stream: String,
    pub user_stream: String,
    pub call_log_since: chrono::DateTime<chrono::Utc>,
}

pub fn spawn<C, W, V, P, D, L>(
    jetstream: async_nats::jetstream::Context,
    spec: VoiceEffectsSpec,
    channel_repository: Arc<C>,
    community_unit_of_work: Arc<W>,
    voice_service: Arc<V>,
    profiles: Arc<P>,
    log_call: Arc<LogCall<D, L>>,
) where
    C: ChannelRepository + Send + Sync + 'static,
    W: CommunityUnitOfWork + Send + Sync + 'static,
    V: VoiceService + Send + Sync + 'static,
    P: UserProfileRepository + Send + Sync + 'static,
    D: DmUnitOfWork + 'static,
    L: CallParticipationLookup + 'static,
{
    let prefix = spec.subject_prefix.trim_end_matches('.').to_string();
    let enforce_mute = Arc::new(EnforceVoiceMute::new(
        channel_repository.clone(),
        community_unit_of_work.clone(),
        voice_service.clone(),
    ));
    let enforce_deactivation = Arc::new(EnforceUserDeactivation::new(
        channel_repository.clone(),
        voice_service.clone(),
        profiles,
    ));
    let resolve_communities = Arc::new(ResolveDeletedUserCommunities::new(
        channel_repository.clone(),
        community_unit_of_work.clone(),
        voice_service.clone(),
    ));
    let enforce_departure = Arc::new(EnforceCommunityDeparture::new(
        community_unit_of_work,
        channel_repository,
        voice_service,
    ));

    {
        let jetstream = jetstream.clone();
        let subject = format!("{prefix}.user.*.call.ended");
        let delivery = call_log_delivery(spec.call_log_since);
        let prefix = prefix.clone();
        let user_stream = spec.user_stream.clone();
        tokio::spawn(async move {
            run(
                jetstream,
                user_stream,
                "messaging-voice-effects-call-log".to_string(),
                subject,
                Vec::new(),
                prefix,
                delivery,
                move |_subject, payload| {
                    let log_call = log_call.clone();
                    async move {
                        let evt: CallEventV1 = serde_json::from_slice(&payload)?;
                        log_call.execute(evt.call_id).await?;
                        Ok(())
                    }
                },
            )
            .await;
        });
    }

    {
        let jetstream = jetstream.clone();
        let subject = format!("{prefix}.user.*.disabled");
        let extra = vec![
            format!("{prefix}.user.*.deleted"),
            format!("{prefix}.user.disabled"),
            format!("{prefix}.user.deleted"),
        ];
        let prefix = prefix.clone();
        let user_stream = spec.user_stream.clone();
        tokio::spawn(async move {
            run(
                jetstream,
                user_stream,
                "messaging-voice-effects-user".to_string(),
                subject,
                extra,
                prefix,
                EFFECT_DELIVERY,
                move |subject, payload| {
                    let enforce_deactivation = enforce_deactivation.clone();
                    let resolve_communities = resolve_communities.clone();
                    async move {
                        if !(subject.ends_with(".disabled") || subject.ends_with(".deleted")) {
                            return Ok(());
                        }
                        let evt: UserLifecycleV1 = serde_json::from_slice(&payload)?;
                        if subject.ends_with(".deleted") {
                            resolve_communities
                                .execute(UserId::new(evt.user_id), evt.occurred_at, None)
                                .await?;
                        }
                        enforce_deactivation
                            .execute(UserId::new(evt.user_id), evt.occurred_at)
                            .await?;
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
            format!("{prefix}.channel.*.participant.deafened"),
            format!("{prefix}.channel.*.participant.undeafened"),
            format!("{prefix}.channel.participant.muted"),
            format!("{prefix}.channel.participant.unmuted"),
            format!("{prefix}.channel.participant.deafened"),
            format!("{prefix}.channel.participant.undeafened"),
        ];
        let prefix = prefix.clone();
        tokio::spawn(async move {
            run(
                jetstream,
                spec.channel_stream,
                "messaging-voice-effects-channel".to_string(),
                subject,
                extra,
                prefix,
                EFFECT_DELIVERY,
                move |subject, payload| {
                    let enforce_mute = enforce_mute.clone();
                    async move {
                        let (channel_id, user_id) = if subject.ends_with(".participant.muted") {
                            let evt: ParticipantMutedV1 = serde_json::from_slice(&payload)?;
                            (evt.channel_id, evt.user_id)
                        } else if subject.ends_with(".participant.unmuted") {
                            let evt: ParticipantUnmutedV1 = serde_json::from_slice(&payload)?;
                            (evt.channel_id, evt.user_id)
                        } else if subject.ends_with(".participant.deafened") {
                            let evt: ParticipantDeafenedV1 = serde_json::from_slice(&payload)?;
                            (evt.channel_id, evt.user_id)
                        } else if subject.ends_with(".participant.undeafened") {
                            let evt: ParticipantUndeafenedV1 = serde_json::from_slice(&payload)?;
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
        let subject = format!("{prefix}.community.*.member_left");
        tokio::spawn(async move {
            run(
                jetstream,
                spec.community_stream,
                "messaging-voice-effects-community".to_string(),
                subject,
                vec![format!("{prefix}.community.member_left")],
                prefix,
                EFFECT_DELIVERY,
                move |subject, payload| {
                    let enforce_departure = enforce_departure.clone();
                    async move {
                        if !subject.ends_with(".member_left") {
                            return Ok(());
                        }
                        let evt: CommunityMemberV1 = serde_json::from_slice(&payload)?;
                        enforce_departure
                            .execute(CommunityId::new(evt.community_id), UserId::new(evt.user_id))
                            .await?;
                        Ok(())
                    }
                },
            )
            .await;
        });
    }
}

#[allow(clippy::too_many_arguments)]
async fn run<F, Fut>(
    jetstream: async_nats::jetstream::Context,
    stream: String,
    consumer_name: String,
    subject: String,
    extra_subjects: Vec<String>,
    prefix: String,
    delivery: Delivery,
    handler: F,
) where
    F: Fn(String, Vec<u8>) -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
{
    let spec = ConsumerSpec {
        stream,
        dlq_subject: dlq_subject(&prefix, &consumer_name),
        consumer_name,
        subjects: std::iter::once(subject).chain(extra_subjects).collect(),
        start_position: delivery.start,
        delivery: RunnerDelivery::new(delivery.max_deliver, delivery.retry_delay).with_ack_wait(ACK_WAIT),
    };
    run_consumer(jetstream, spec, Lifecycle::default(), |event: Event| handler(event.subject, event.payload)).await;
}
