use application::ports::event_consumer::{
    ConsumedMessage, EventConsumer, EventConsumerError, StartPosition, SubscribeOptions,
};
use async_nats::jetstream::{
    Context,
    consumer::{AckPolicy, Consumer, DeliverPolicy, pull},
};
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt, stream::BoxStream};

pub struct NatsEventConsumer {
    jetstream: Context,
    stream_name: String,
}

impl NatsEventConsumer {
    pub fn new(jetstream: Context, stream_name: String) -> Self {
        Self {
            jetstream,
            stream_name,
        }
    }
}

impl EventConsumer for NatsEventConsumer {
    fn subscribe(
        &self,
        opts: SubscribeOptions,
    ) -> BoxFuture<
        '_,
        Result<BoxStream<'static, Result<ConsumedMessage, EventConsumerError>>, EventConsumerError>,
    > {
        let jetstream = self.jetstream.clone();
        let stream_name = self.stream_name.clone();
        let SubscribeOptions {
            consumer_name,
            subject,
            start_position,
            max_deliver,
            dlq_subject,
        } = opts;
        let deliver_policy = match start_position {
            StartPosition::New => DeliverPolicy::New,
            StartPosition::All => DeliverPolicy::All,
        };
        let max_deliver_i64: i64 = match (max_deliver, dlq_subject.as_ref()) {
            (Some(n), None) => n as i64,
            _ => -1,
        };

        async move {
            let nats_consumer = create_durable_consumer(
                &jetstream,
                &stream_name,
                &consumer_name,
                subject,
                deliver_policy,
                max_deliver_i64,
            )
            .await
            .map_err(|e| EventConsumerError::Subscribe(e.to_string()))?;

            let messages = nats_consumer
                .messages()
                .await
                .map_err(|e| EventConsumerError::Subscribe(e.to_string()))?;

            let dlq_publisher = jetstream.clone();
            let mapped = messages.filter_map(move |result| {
                let dlq_publisher = dlq_publisher.clone();
                let dlq_subject = dlq_subject.clone();
                async move {
                    match result {
                        Ok(msg) => {
                            if let (Some(max), Some(dlq)) = (max_deliver, dlq_subject.as_ref()) {
                                let delivered =
                                    msg.info().map(|i| i.delivered as u32).unwrap_or(0);
                                if delivered > max {
                                    let ack_future = match dlq_publisher
                                        .publish(dlq.clone(), msg.payload.clone())
                                        .await
                                    {
                                        Ok(f) => f,
                                        Err(e) => {
                                            return Some(Err(EventConsumerError::Stream(
                                                format!("DLQ publish failed: {}", e),
                                            )));
                                        }
                                    };
                                    if let Err(e) = ack_future.await {
                                        return Some(Err(EventConsumerError::Stream(format!(
                                            "DLQ publish ack failed: {}",
                                            e
                                        ))));
                                    }
                                    if let Err(e) = msg.ack().await {
                                        tracing::error!(
                                            error = %e,
                                            "Failed to ack message after DLQ publish",
                                        );
                                    } else {
                                        tracing::warn!(
                                            dlq = %dlq,
                                            delivered = delivered,
                                            "Message moved to DLQ after max delivery attempts",
                                        );
                                    }
                                    return None;
                                }
                            }
                            let payload = msg.payload.to_vec();
                            let ack_fn: Box<
                                dyn FnOnce() -> BoxFuture<
                                        'static,
                                        Result<(), EventConsumerError>,
                                    > + Send,
                            > = Box::new(move || {
                                async move {
                                    msg.ack()
                                        .await
                                        .map_err(|e| EventConsumerError::Ack(e.to_string()))
                                }
                                .boxed()
                            });
                            Some(Ok(ConsumedMessage::new(payload, ack_fn)))
                        }
                        Err(e) => Some(Err(EventConsumerError::Stream(e.to_string()))),
                    }
                }
            });

            Ok(Box::pin(mapped) as BoxStream<'static, _>)
        }
        .boxed()
    }
}

async fn create_durable_consumer(
    jetstream: &Context,
    stream_name: &str,
    consumer_name: &str,
    filter_subject: String,
    deliver_policy: DeliverPolicy,
    max_deliver: i64,
) -> Result<Consumer<pull::Config>, async_nats::Error> {
    let stream = jetstream.get_stream(stream_name).await?;

    let consumer = stream
        .create_consumer(pull::Config {
            durable_name: Some(consumer_name.to_string()),
            filter_subject,
            ack_policy: AckPolicy::Explicit,
            deliver_policy,
            max_deliver,
            ..Default::default()
        })
        .await?;

    Ok(consumer)
}
