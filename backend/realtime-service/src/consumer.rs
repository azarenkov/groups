use std::time::Duration;

use async_nats::jetstream::{
    Context,
    consumer::{AckPolicy, DeliverPolicy, pull},
};
use futures::StreamExt;

use crate::{
    connections::Publisher, dispatch::dispatch, recipients::RecipientResolver,
};

#[derive(Clone)]
pub struct ConsumerSpec {
    pub stream: String,
    pub consumer_name: String,
    pub filter_subject: String,
}

pub async fn run_consumer(
    jetstream: Context,
    spec: ConsumerSpec,
    resolver: RecipientResolver,
    publisher: Publisher,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    loop {
        if *shutdown.borrow() {
            return;
        }

        match subscribe(&jetstream, &spec).await {
            Ok(mut stream) => {
                tracing::info!(
                    stream = %spec.stream,
                    consumer = %spec.consumer_name,
                    subject = %spec.filter_subject,
                    "Consumer subscribed",
                );
                loop {
                    tokio::select! {
                        _ = shutdown.changed() => return,
                        msg = stream.next() => {
                            match msg {
                                Some(Ok(m)) => {
                                    let subject = m.subject.to_string();
                                    let payload = m.payload.clone();
                                    match dispatch(&subject, &payload, &resolver, &publisher).await {
                                        Ok(_) => {
                                            if let Err(e) = m.ack().await {
                                                tracing::warn!(error = %e, %subject, "Failed to ack message");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, %subject, "Dispatch failed, not acking");
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

async fn subscribe(
    jetstream: &Context,
    spec: &ConsumerSpec,
) -> Result<futures::stream::BoxStream<'static, Result<async_nats::jetstream::Message, async_nats::Error>>, async_nats::Error> {
    let stream = jetstream.get_stream(&spec.stream).await?;
    let consumer = stream
        .create_consumer(pull::Config {
            durable_name: Some(spec.consumer_name.clone()),
            filter_subject: spec.filter_subject.clone(),
            ack_policy: AckPolicy::Explicit,
            deliver_policy: DeliverPolicy::New,
            ..Default::default()
        })
        .await?;
    let messages = consumer
        .messages()
        .await?
        .map(|r| r.map_err(|e| async_nats::Error::from(e.to_string())));
    Ok(Box::pin(messages))
}
