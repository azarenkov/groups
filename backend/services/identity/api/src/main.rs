mod dto;
mod http_errors;
mod auth;
mod internal;
mod routes;
mod server;
mod state;
mod activity;
mod appearance;
mod connections;
mod user;
mod notification_settings;
mod voice_settings;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    crate::server::run().await
}
