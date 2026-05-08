use application::ports::event_consumer::EventConsumer;
use async_nats::jetstream::Context;

pub struct NatsConsumer {
    jetstream: Context,
    subject_prefix: String,
}

impl NatsConsumer {
    pub fn new(jetstream: Context, subject_prefix: String) -> Self {
        Self {
            jetstream,
            subject_prefix,
        }
    }
}

// impl<T> EventConsumer<T> for NatsConsumer {}
