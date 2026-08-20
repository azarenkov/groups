use std::sync::Arc;

use application::ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use domain::{user::id::UserId, user_profile::UserProfileRepository};
use futures::StreamExt;
use infrastructure::messaging::nats::consumer::NatsEventConsumer;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct UserNameDto {
    user_id: Uuid,
    name: String,
    occurred_at: chrono::DateTime<chrono::Utc>,
}

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
            };
            match consumer.subscribe(opts).await {
                Ok(mut messages) => {
                    tracing::info!(%stream, consumer_name, "User profile projection subscribed");
                    while let Some(msg) = messages.next().await {
                        match msg {
                            Ok(m) => {
                                let relevant = m.subject.ends_with(".user.registered")
                                    || m.subject.ends_with(".user.renamed");
                                if !relevant {
                                    let _ = m.ack().await;
                                    continue;
                                }
                                match serde_json::from_slice::<UserNameDto>(&m.payload) {
                                    Ok(evt) => {
                                        match repository
                                            .upsert_name(&UserId::new(evt.user_id), &evt.name, evt.occurred_at)
                                            .await
                                        {
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
