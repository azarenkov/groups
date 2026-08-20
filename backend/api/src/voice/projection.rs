use std::sync::Arc;

use application::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use domain::{user::id::UserId, user_profile::UserProfileRepository};
use futures::StreamExt;
use infrastructure::messaging::nats::consumer::NatsEventConsumer;
use contracts::events::user::UserNameV1 as UserNameDto;

pub fn spawn<P>(
    jetstream: async_nats::jetstream::Context,
    subject_prefix: String,
    stream: String,
    profiles: Arc<P>,
) where
    P: UserProfileRepository + Send + Sync + 'static,
{
    actix_web::rt::spawn(async move {
        let prefix = subject_prefix.trim_end_matches('.').to_string();
        let consumer = NatsEventConsumer::new(jetstream, stream.clone());
        loop {
            let opts = SubscribeOptions {
                consumer_name: "voice-projections".to_string(),
                subject: format!("{prefix}.user.>"),
                start_position: StartPosition::All,
                max_deliver: Some(5),
                dlq_subject: Some(format!("{prefix}.dlq.voice.projections")),
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, "Voice projection consumer subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
                                match handle(&m.subject, &m.payload, &*profiles).await {
                                    Ok(()) => {
                                        if let Err(e) = m.ack().await {
                                            tracing::warn!(error = %e, "Voice projection ack failed");
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, "Voice projection failed; message will be redelivered");
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Voice projection stream error");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Voice projection subscribe failed; retrying");
                }
            }
            actix_web::rt::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
}

async fn handle<P>(
    subject: &str,
    payload: &[u8],
    profiles: &P,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    P: UserProfileRepository + Send + Sync,
{
    if subject.ends_with(".user.registered") || subject.ends_with(".user.renamed") {
        match serde_json::from_slice::<UserNameDto>(payload) {
            Ok(evt) => {
                profiles
                    .upsert_name(&UserId::new(evt.user_id), &evt.name, evt.occurred_at)
                    .await?;
            }
            Err(e) => {
                tracing::warn!(error = %e, %subject, "Malformed user event; dropping");
            }
        }
        return Ok(());
    }
    Ok(())
}
