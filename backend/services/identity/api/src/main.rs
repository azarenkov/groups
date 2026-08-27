mod dto;
mod http_errors;
mod auth;
mod internal;
mod routes;
mod server;
mod state;
mod user;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    crate::server::run().await
}
