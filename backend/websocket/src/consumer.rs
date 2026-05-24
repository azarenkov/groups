use std::time::Duration;

use application::ports::event_consumer::{
    EventConsumer, StartPosition, SubscribeOptions,
};
use futures::StreamExt;
use infrastructure::messaging::nats::consumer::NatsEventConsumer;

use crate::{
    connections::Publisher, dispatch::dispatch, recipients::RecipientResolver,
};

#[derive(Clone)]
pub struct ConsumerSpec {
    pub stream: String,
    pub consumer_name: String,
    pub filter_subject: String,
    pub dlq_subject: String,
    pub max_deliver: u32,
}

pub async fn run_consumer(
    jetstream: async_nats::jetstream::Context,
    spec: ConsumerSpec,
    resolver: RecipientResolver,
    publisher: Publisher,
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
            start_position: StartPosition::New,
            max_deliver: Some(spec.max_deliver),
            dlq_subject: Some(spec.dlq_subject.clone()),
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
                                    match dispatch(&subject, &m.payload, &resolver, &publisher).await {
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
