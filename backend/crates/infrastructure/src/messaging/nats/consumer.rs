use application::ports::event_consumer::{
    ConsumedMessage, EventConsumer, EventConsumerError, StartPosition,
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
        consumer_name: &str,
        subject: &str,
        start_position: StartPosition,
    ) -> BoxFuture<
        '_,
        Result<BoxStream<'static, Result<ConsumedMessage, EventConsumerError>>, EventConsumerError>,
    > {
        let jetstream = self.jetstream.clone();
        let stream_name = self.stream_name.clone();
        let consumer_name = consumer_name.to_string();
        let subject = subject.to_string();
        let deliver_policy = match start_position {
            StartPosition::New => DeliverPolicy::New,
            StartPosition::All => DeliverPolicy::All,
        };

        async move {
            let nats_consumer = create_durable_consumer(
                &jetstream,
                &stream_name,
                &consumer_name,
                subject,
                deliver_policy,
            )
            .await
            .map_err(|e| EventConsumerError::Subscribe(e.to_string()))?;

            let messages = nats_consumer
                .messages()
                .await
                .map_err(|e| EventConsumerError::Subscribe(e.to_string()))?;

            let mapped = messages.map(|result| match result {
                Ok(msg) => {
                    let payload = msg.payload.to_vec();
                    let ack_fn: Box<
                        dyn FnOnce() -> BoxFuture<'static, Result<(), EventConsumerError>> + Send,
                    > = Box::new(move || {
                        async move {
                            msg.ack()
                                .await
                                .map_err(|e| EventConsumerError::Ack(e.to_string()))
                        }
                        .boxed()
                    });
                    Ok(ConsumedMessage::new(payload, ack_fn))
                }
                Err(e) => Err(EventConsumerError::Stream(e.to_string())),
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
) -> Result<Consumer<pull::Config>, async_nats::Error> {
    let stream = jetstream.get_stream(stream_name).await?;

    let consumer = stream
        .get_or_create_consumer(
            consumer_name,
            pull::Config {
                durable_name: Some(consumer_name.to_string()),
                filter_subject,
                ack_policy: AckPolicy::Explicit,
                deliver_policy,
                ..Default::default()
            },
        )
        .await?;

    Ok(consumer)
}
