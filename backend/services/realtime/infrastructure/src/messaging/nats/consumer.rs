use std::time::Duration;

use ports::event_consumer::{
    EventConsumer, StartPosition, SubscribeOptions,
};
use futures::StreamExt;
use tracing::Instrument;
use infrastructure_common::messaging::nats::consumer::NatsEventConsumer;

use std::sync::Arc;

use realtime_core::{ports::directory::RecipientDirectory, ports::projection::RealtimeProjection, ports::publisher::RealtimePublisher, usecases::dispatch_event::DispatchEvent};

#[derive(Clone)]
pub struct ConsumerSpec {
    pub stream: String,
    pub consumer_name: String,
    pub filter_subject: String,
    pub dlq_subject: String,
    pub max_deliver: u32,
    pub start_position: StartPosition,
}

pub async fn run_consumer<D: RecipientDirectory + RealtimeProjection + 'static, P: RealtimePublisher + 'static>(
    jetstream: async_nats::jetstream::Context,
    spec: ConsumerSpec,
    dispatch: Arc<DispatchEvent<D, P>>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let event_consumer =
        NatsEventConsumer::new(jetstream.clone(), spec.stream.clone());

    loop {
        if *shutdown.borrow() {
            return;
        }

        let opts = SubscribeOptions {
            consumer_name: spec.consumer_name.clone(),
            subject: spec.filter_subject.clone(),
            start_position: spec.start_position,
            max_deliver: Some(spec.max_deliver),
            dlq_subject: Some(spec.dlq_subject.clone()),
            extra_subjects: vec![],
        };

        match event_consumer.subscribe(opts).await {
            Ok(mut messages) => {
                tracing::info!(
                    stream = %spec.stream,
                    consumer = %spec.consumer_name,
                    subject = %spec.filter_subject,
                    dlq = %spec.dlq_subject,
                    max_deliver = spec.max_deliver,
                    "Consumer subscribed",
                );
                loop {
                    tokio::select! {
                        _ = shutdown.changed() => return,
                        msg = messages.next() => {
                            match msg {
                                Some(Ok(m)) => {
                                    let subject = m.subject.clone();
                                    let span = tracing::info_span!(
                                        "event",
                                        request_id = ?m.request_id,
                                        subject = %subject,
                                    );
                                    async {
                                        match dispatch.execute(&subject, &m.payload).await {
                                            Ok(_) => {
                                                if let Err(e) = m.ack().await {
                                                    tracing::warn!(error = %e, %subject, "Failed to ack message");
                                                }
                                            }
                                            Err(e) => {
                                                tracing::warn!(error = %e, %subject, "Dispatch failed; not acking, JetStream will redeliver");
                                            }
                                        }
                                    }
                                    .instrument(span)
                                    .await;
                                }
                                Some(Err(e)) => {
                                    tracing::error!(error = %e, stream = %spec.stream, "Consumer stream error");
                                    break;
                                }
                                None => {
                                    tracing::warn!(stream = %spec.stream, "Consumer stream ended");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, stream = %spec.stream, "Subscribe failed");
            }
        }

        tokio::select! {
            _ = shutdown.changed() => return,
            _ = tokio::time::sleep(Duration::from_secs(3)) => {}
        }
    }
}
