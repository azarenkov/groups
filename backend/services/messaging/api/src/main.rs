mod dto;
mod http_errors;
mod bootstrap;
mod attachments;
mod emoji_status;
mod blocks;
mod channel;
mod dm;
mod dm_conversations;
mod forward;
mod friendship;
mod home;
mod housekeeping;
mod inbox;
mod internal;
mod invites;
mod notifications;
mod pins;
mod privacy;
mod read_state;
mod reports;
mod roles;
mod routes;
mod server;
mod search;
mod shared;
mod voice;
mod state;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = api_common::health::run_probe_from_args() {
        std::process::exit(code);
    }
    crate::bootstrap::run().await
}
