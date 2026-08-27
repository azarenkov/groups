use ports::event_consumer::{EventConsumer, StartPosition, SubscribeOptions};
use futures::StreamExt;
use tracing::Instrument;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;

use std::sync::Arc;

use realtime_core::{ports::projection::RealtimeProjection, usecases::apply_projection::ApplyProjection};

pub struct CatchupSpec {
    pub stream: String,
    pub consumer_name: String,
    pub filter_subject: String,
    pub dlq_subject: String,
}

pub async fn run_catchup<R: RealtimeProjection + 'static>(
    jetstream: async_nats::jetstream::Context,
    spec: CatchupSpec,
    apply: Arc<ApplyProjection<R>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let consumer = NatsEventConsumer::new(jetstream, spec.stream.clone());
    loop {
        if *shutdown.borrow() {
            return;
        }
        let opts = SubscribeOptions {
            consumer_name: spec.consumer_name.clone(),
            subject: spec.filter_subject.clone(),
            start_position: StartPosition::All,
            max_deliver: Some(5),
            dlq_subject: Some(spec.dlq_subject.clone()),
            extra_subjects: vec![],
            ack_wait: None,
        };
        match consumer.subscribe(opts).await {
            Ok(mut messages) => {
                tracing::info!(
                    stream = %spec.stream,
                    consumer = %spec.consumer_name,
                    "Projection catch-up consumer subscribed",
                );
                loop {
                    tokio::select! {
                        _ = shutdown.changed() => return,
                        msg = messages.next() => {
                            match msg {
                                Some(Ok(m)) => {
                                    let span = tracing::info_span!(
                                        "event",
                                        request_id = ?m.request_id,
                                        subject = %m.subject,
                                    );
                                    async {
                                        match apply.execute(&m.subject, &m.payload).await {
                                            Ok(()) => {
                                                if let Err(e) = m.ack().await {
                                                    tracing::warn!(error = %e, "Projection catch-up ack failed");
                                                }
                                            }
                                            Err(e) => {
                                                tracing::warn!(error = %e, "Projection catch-up failed; message will be redelivered");
                                            }
                                        }
                                    }
                                    .instrument(span)
                                    .await;
                                }
                                Some(Err(e)) => {
                                    tracing::warn!(error = %e, "Projection catch-up stream error");
                                    break;
                                }
                                None => break,
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Projection catch-up subscribe failed; retrying");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
