mod dto;
mod http_errors;
mod bootstrap;
mod attachments;
mod badges;
mod blocks;
mod calls;
mod channel;
mod dm;
mod forward;
mod friendship;
mod home;
mod internal;
mod invites;
mod notifications;
mod presence;
mod privacy;
mod read_state;
mod reports;
mod roles;
mod routes;
mod server;
mod state;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = api_common::health::run_probe_from_args() {
        std::process::exit(code);
    }
    crate::bootstrap::run().await
}
