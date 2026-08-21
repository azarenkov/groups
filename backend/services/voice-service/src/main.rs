mod call;
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
