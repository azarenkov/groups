mod call;
mod http_errors;
mod internal;
mod projection;
mod routes;
mod server;
mod state;
mod webhook;

use std::error::Error;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    crate::server::run().await
}
