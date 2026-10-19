mod dto;
mod http_errors;
mod auth;
mod internal;
mod routes;
mod server;
mod state;
mod appearance;
mod link_preview;
mod user;
mod notification_settings;
mod hotkey_settings;
mod voice_settings;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Some(code) = api_common::health::run_probe_from_args() {
        std::process::exit(code);
    }
    crate::server::run().await
}
