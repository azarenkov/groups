use std::{error::Error, time::Duration};

use async_nats::{
    ConnectOptions, ServerAddr,
    connection::State,
    jetstream::{self, Context, ErrorCode, context::GetStreamErrorKind},
};

const STREAM_NOT_FOUND_CODE: ErrorCode = ErrorCode(10059);

use crate::config::nats::NatsConfig;

const DUPLICATE_WINDOW_SECS: u64 = 60 * 10;

#[derive(Clone, Copy)]
enum StreamTier {
    Hot,
    Cold,
}

#[derive(Clone, Copy)]
enum StreamDurability {
    Expiring,
    UntilConsumed,
}

pub mod streams {
    pub const USER_EVENTS: &str = "RIFT_USER_EVENTS";
    pub const CHANNEL_EVENTS: &str = "RIFT_CHANNEL_EVENTS";
    pub const MESSAGE_EVENTS: &str = "RIFT_MESSAGE_EVENTS";
    pub const DM_MESSAGE_EVENTS: &str = "RIFT_DM_MESSAGE_EVENTS";
    pub const COMMUNITY_EVENTS: &str = "RIFT_COMMUNITY_EVENTS";
    pub const STORAGE_EVENTS: &str = "RIFT_STORAGE_EVENTS";
    pub const DLQ: &str = "RIFT_DLQ";
}

pub fn stream_name(env_var: &str, default: &str) -> String {
    std::env::var(env_var).unwrap_or_else(|_| default.to_string())
}

struct StreamDef {
    name: &'static str,
    subject_suffix: &'static str,
    max_age_secs: u64,
    tier: StreamTier,
    durability: StreamDurability,
}

const STREAMS: &[StreamDef] = &[
    StreamDef {
        name: streams::USER_EVENTS,
        subject_suffix: "user.>",
        max_age_secs: 60 * 60 * 24 * 365,
        tier: StreamTier::Hot,
        durability: StreamDurability::Expiring,
    },
    StreamDef {
        name: streams::CHANNEL_EVENTS,
        subject_suffix: "channel.>",
        max_age_secs: 60 * 60 * 24 * 30,
        tier: StreamTier::Hot,
        durability: StreamDurability::Expiring,
    },
    StreamDef {
        name: streams::MESSAGE_EVENTS,
        subject_suffix: "message.>",
        max_age_secs: 60 * 60 * 24 * 30,
        tier: StreamTier::Hot,
        durability: StreamDurability::Expiring,
    },
    StreamDef {
        name: streams::DM_MESSAGE_EVENTS,
        subject_suffix: "dm_message.>",
        max_age_secs: 60 * 60 * 24 * 30,
        tier: StreamTier::Hot,
        durability: StreamDurability::Expiring,
    },
    StreamDef {
        name: streams::COMMUNITY_EVENTS,
        subject_suffix: "community.>",
        max_age_secs: 60 * 60 * 24 * 180,
        tier: StreamTier::Cold,
        durability: StreamDurability::Expiring,
    },
    StreamDef {
        name: streams::STORAGE_EVENTS,
        subject_suffix: "storage.>",
        max_age_secs: 0,
        tier: StreamTier::Hot,
        durability: StreamDurability::UntilConsumed,
    },
    StreamDef {
        name: streams::DLQ,
        subject_suffix: "dlq.>",
        max_age_secs: 60 * 60 * 24 * 90,
        tier: StreamTier::Cold,
        durability: StreamDurability::Expiring,
    },
];

fn credentials(addr: &ServerAddr) -> Option<(String, String)> {
    match (addr.username(), addr.password()) {
        (Some(user), Some(pass)) => Some((user.to_string(), pass.to_string())),
        _ => None,
    }
}

pub async fn connect(url: &str) -> Result<async_nats::Client, Box<dyn Error>> {
    let addr: ServerAddr = url.parse()?;
    let options = match credentials(&addr) {
        Some((user, pass)) => ConnectOptions::with_user_and_password(user, pass),
        None => ConnectOptions::new(),
    };
    Ok(options.connect(addr).await?)
}

pub async fn create_jetstream(config: &NatsConfig) -> Result<Context, Box<dyn Error>> {
    let client = connect(config.url()).await?;
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
        let max_bytes = match (def.durability, def.tier) {
            (StreamDurability::UntilConsumed, _) => -1,
            (StreamDurability::Expiring, StreamTier::Hot) => config.stream_hot_max_bytes,
            (StreamDurability::Expiring, StreamTier::Cold) => config.stream_cold_max_bytes,
        };
        let (retention, discard) = match def.durability {
            StreamDurability::Expiring => (
                jetstream::stream::RetentionPolicy::Limits,
                jetstream::stream::DiscardPolicy::Old,
            ),
            StreamDurability::UntilConsumed => (
                jetstream::stream::RetentionPolicy::WorkQueue,
                jetstream::stream::DiscardPolicy::New,
            ),
        };
        let stream_config = jetstream::stream::Config {
            name: def.name.to_string(),
            subjects: vec![subject_pattern],
            storage: jetstream::stream::StorageType::File,
            max_age: Duration::from_secs(def.max_age_secs),
            max_bytes,
            duplicate_window: Duration::from_secs(DUPLICATE_WINDOW_SECS),
            retention,
            discard,
            compression: Some(jetstream::stream::Compression::S2),
            num_replicas: config.replicas,
            ..Default::default()
        };

        match jetstream.get_stream(def.name).await {
            Ok(_) => {
                jetstream.update_stream(&stream_config).await?;
                tracing::debug!(
                    name = def.name,
                    max_bytes,
                    "JetStream stream configuration updated",
                );
            }
            Err(e) if is_stream_not_found(&e) => {
                jetstream.create_stream(stream_config).await?;
                tracing::info!(
                    name = def.name,
                    max_bytes,
                    "JetStream stream created",
                );
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    Ok(())
}

fn is_stream_not_found(err: &async_nats::jetstream::context::GetStreamError) -> bool {
    matches!(
        err.kind(),
        GetStreamErrorKind::JetStream(ref e) if e.error_code() == STREAM_NOT_FOUND_CODE,
    )
}

pub fn is_nats_connected(jetstream: &Context) -> bool {
    matches!(jetstream.client().connection_state(), State::Connected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_come_from_the_url() {
        let addr: ServerAddr = "nats://rift:s3cr3t@nats.nats.svc:4222".parse().unwrap();
        assert_eq!(credentials(&addr), Some(("rift".to_string(), "s3cr3t".to_string())));
    }

    #[test]
    fn plain_urls_have_no_credentials() {
        let addr: ServerAddr = "nats://nats:4222".parse().unwrap();
        assert_eq!(credentials(&addr), None);
    }
}
