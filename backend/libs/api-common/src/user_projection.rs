use std::sync::Arc;

use domain::user::id::UserId;
use ports::{event_consumer::EventConsumer, event_consumer::StartPosition, event_consumer::SubscribeOptions, user_profile::UserProfileRepository};
use futures::StreamExt;
use tracing::Instrument;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;
use contracts::events::user::{UserLifecycleV1 as UserLifecycleDto, UserNameV1 as UserNameDto, UserProfileChangedV1 as UserProfileChangedDto};

pub fn spawn<P>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    consumer_name: &'static str,
    repository: Arc<P>,
) where
    P: UserProfileRepository + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: consumer_name.to_string(),
                subject: format!("{prefix}.user.>"),
                start_position: StartPosition::All,
                max_deliver: Some(5),
                dlq_subject: Some(format!("{prefix}.dlq.{consumer_name}")),
                extra_subjects: vec![],
                ack_wait: None,
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, consumer_name, "User profile projection subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
                                let name_event = m.subject.ends_with(".user.registered")
                                    || m.subject.ends_with(".user.renamed");
                                let active_state = if m.subject.ends_with(".user.disabled")
                                    || m.subject.ends_with(".user.deleted")
                                {
                                    Some(false)
                                } else if m.subject.ends_with(".user.reactivated") {
                                    Some(true)
                                } else {
                                    None
                                };
                                let profile_event = m.subject.ends_with(".user.profile_changed");
                                if !name_event && active_state.is_none() && !profile_event {
                                    let _ = m.ack().await;
                                    continue;
                                }
                                if profile_event {
                                    match serde_json::from_slice::<UserProfileChangedDto>(&m.payload) {
                                        Ok(evt) => match repository
                                            .update_profile(&UserId::new(evt.user_id), &evt.bio, evt.show_activity, evt.active, &evt.timezone, evt.occurred_at)
                                            .await
                                        {
                                            Ok(()) => {
                                                let _ = m.ack().await;
                                            }
                                            Err(e) => {
                                                tracing::warn!(error = %e, "Profile update failed; message will be redelivered");
                                            }
                                        },
                                        Err(e) => {
                                            tracing::warn!(error = %e, subject = %m.subject, "Malformed user profile event; dropping");
                                            let _ = m.ack().await;
                                        }
                                    }
                                    continue;
                                }
                                if let Some(active) = active_state {
                                    match serde_json::from_slice::<UserLifecycleDto>(&m.payload) {
                                        Ok(evt) => match repository
                                            .set_active(&UserId::new(evt.user_id), active, evt.occurred_at)
                                            .await
                                        {
                                            Ok(_) => {
                                                if m.subject.ends_with(".user.deleted")
                                                    && let Err(e) = repository
                                                        .update_profile(&UserId::new(evt.user_id), "", false, false, "", evt.occurred_at)
                                                        .await
                                                {
                                                    tracing::warn!(error = %e, "Profile erasure failed; message will be redelivered");
                                                    continue;
                                                }
                                                let _ = m.ack().await;
                                            }
                                            Err(e) => {
                                                tracing::warn!(error = %e, "Profile active-state update failed; message will be redelivered");
                                            }
                                        },
                                        Err(e) => {
                                            tracing::warn!(error = %e, subject = %m.subject, "Malformed user lifecycle event; dropping");
                                            let _ = m.ack().await;
                                        }
                                    }
                                    continue;
                                }
                                let span = tracing::info_span!(
                                    "event",
                                    request_id = ?m.request_id,
                                    subject = %m.subject,
                                );
                                async {
                                    match serde_json::from_slice::<UserNameDto>(&m.payload) {
                                        Ok(evt) => {
                                            let registered = if m.subject.ends_with(".user.registered") {
                                                repository
                                                    .set_registered_at(&UserId::new(evt.user_id), evt.occurred_at)
                                                    .await
                                            } else {
                                                Ok(())
                                            };
                                            match registered.and(
                                                repository
                                                    .upsert_name(&UserId::new(evt.user_id), &evt.name, evt.occurred_at)
                                                    .await,
                                            ) {
                                                Ok(()) => {
                                                    if let Err(e) = m.ack().await {
                                                        tracing::warn!(error = %e, "Profile projection ack failed");
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::warn!(error = %e, "Profile projection upsert failed; message will be redelivered");
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, subject = %m.subject, "Malformed user event; dropping");
                                            let _ = m.ack().await;
                                        }
                                    }
                                }
                                .instrument(span)
                                .await;
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Profile projection stream error");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Profile projection subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}
