use std::{error::Error, time::Duration};

use async_nats::{
    connection::State,
    jetstream::{self, Context},
};

use crate::config::nats::NatsConfig;

const DUPLICATE_WINDOW_SECS: u64 = 60 * 10;

struct StreamDef {
    name: &'static str,
    subject_suffix: &'static str,
    max_age_secs: u64,
    max_bytes: i64,
}

const STREAMS: &[StreamDef] = &[
    StreamDef {
        name: "RIFT_USER_EVENTS",
        subject_suffix: "user.>",
        max_age_secs: 60 * 60 * 24 * 365,
        max_bytes: 50 * 1024 * 1024 * 1024,
    },
    StreamDef {
        name: "RIFT_CHANNEL_EVENTS",
        subject_suffix: "channel.>",
        max_age_secs: 60 * 60 * 24 * 30,
        max_bytes: 50 * 1024 * 1024 * 1024,
    },
    StreamDef {
        name: "RIFT_MESSAGE_EVENTS",
        subject_suffix: "message.>",
        max_age_secs: 60 * 60 * 24 * 30,
        max_bytes: 50 * 1024 * 1024 * 1024,
    },
    StreamDef {
        name: "RIFT_DM_MESSAGE_EVENTS",
        subject_suffix: "dm_message.>",
        max_age_secs: 60 * 60 * 24 * 30,
        max_bytes: 50 * 1024 * 1024 * 1024,
    },
    StreamDef {
        name: "RIFT_SERVER_EVENTS",
        subject_suffix: "server.>",
        max_age_secs: 60 * 60 * 24 * 180,
        max_bytes: 10 * 1024 * 1024 * 1024,
    },
    StreamDef {
        name: "RIFT_DLQ",
        subject_suffix: "dlq.>",
        max_age_secs: 60 * 60 * 24 * 90,
        max_bytes: 10 * 1024 * 1024 * 1024,
    },
];

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
    for def in STREAMS {
        let subject_pattern = format!("{}.{}", config.subject_prefix, def.subject_suffix);

        jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: def.name.to_string(),
                subjects: vec![subject_pattern],
                storage: jetstream::stream::StorageType::File,
                max_age: Duration::from_secs(def.max_age_secs),
                max_bytes: def.max_bytes,
                duplicate_window: Duration::from_secs(DUPLICATE_WINDOW_SECS),
                discard: jetstream::stream::DiscardPolicy::Old,
                compression: Some(jetstream::stream::Compression::S2),
                num_replicas: config.replicas,
                ..Default::default()
            })
            .await?;
    }

    Ok(())
}

pub fn is_nats_connected(jetstream: &Context) -> bool {
    matches!(jetstream.client().connection_state(), State::Connected)
}
