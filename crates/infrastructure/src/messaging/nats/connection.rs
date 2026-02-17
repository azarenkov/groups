use std::error::Error;

use async_nats::jetstream::{self, Context};

use crate::config::nats::NatsConfig;

pub async fn create_jetstream(config: &NatsConfig) -> Result<Context, Box<dyn Error>> {
    let client = async_nats::connect(config.url()).await?;
    let jetstream = jetstream::new(client);
    Ok(jetstream)
}
