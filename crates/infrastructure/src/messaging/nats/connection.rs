use std::error::Error;

use async_nats::jetstream::{self, Context};

use crate::config::nats::NatsConfig;

const MAX_AGE_HOURS: u64 = 2190;

pub async fn create_jetstream(config: &NatsConfig) -> Result<Context, Box<dyn Error>> {
    let client = async_nats::connect(config.url()).await?;
    let jetstream = jetstream::new(client);

    initialize_streams(&jetstream, config).await?;

    Ok(jetstream)
}

async fn initialize_streams(
    jetstream: &Context,
    config: &NatsConfig,
) -> Result<(), Box<dyn Error>> {
    let subject_pattern = format!("{}.>", config.subject_prefix);

    jetstream
        .get_or_create_stream(jetstream::stream::Config {
            name: "ROOM_EVENTS".to_string(),
            subjects: vec![subject_pattern],
            storage: jetstream::stream::StorageType::File,
            max_age: std::time::Duration::from_hours(MAX_AGE_HOURS),
            discard: jetstream::stream::DiscardPolicy::Old,
            compression: Some(jetstream::stream::Compression::S2),
            num_replicas: config.replicas,
            ..Default::default()
        })
        .await?;

    Ok(())
}
